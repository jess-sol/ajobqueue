use std::{marker::PhantomData, sync::{RwLock, Arc}, collections::HashMap};

use async_channel::{unbounded, Receiver, Sender};
use async_trait::async_trait;
use log::info;
use serde::de::DeserializeOwned;
use ulid::Ulid;

use super::{StorageProvider, JobMetadata, JobState, JobInfo};
use crate::{
    error::{JobRunError, StorageError},
    JobTypeMarker,
};

// PhantomData necessary so struct only impls one generic impl of StorageProvider
pub struct InMemoryStorageProvider<J: JobTypeMarker + ?Sized> {
    job_queue: (Sender<(Ulid, String)>, Receiver<(Ulid, String)>),
    jobs: Arc<RwLock<HashMap<Ulid, JobMetadata>>>,
    _phantom_data: PhantomData<J>,
}

impl<J: JobTypeMarker + ?Sized> Clone for InMemoryStorageProvider<J> {
    fn clone(&self) -> Self {
        Self {
            job_queue: self.job_queue.clone(),
            jobs: self.jobs.clone(),
            _phantom_data: PhantomData,
        }
    }
}

impl<J: JobTypeMarker + ?Sized> InMemoryStorageProvider<J> {
    pub fn new() -> Self {
        InMemoryStorageProvider {
            job_queue: unbounded(),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            _phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<J: JobTypeMarker + ?Sized> StorageProvider<J> for InMemoryStorageProvider<J>
where Box<J>: DeserializeOwned
{
    async fn pull(&mut self) -> Result<JobInfo<J>, StorageError> {
        let (uid, serialized_job) = self.job_queue.1.recv().await
            .map_err(|x| StorageError::Unspecified(x.to_string()))?;
        let job: Box<J> = serde_json::from_str(&serialized_job)?;


        let jobs = self.jobs.read()
            .map_err(|x| StorageError::Unspecified(x.to_string()))?;
        let metadata = jobs.get(&uid)
            .ok_or_else(|| StorageError::Unspecified(format!("Uid not found: {}", uid)))?;

        Ok(JobInfo {
            metadata: metadata.clone(),
            job,
        })
    }

    async fn push(&mut self, job: &J) -> Result<JobMetadata, StorageError> {
        let uid = Ulid::new();
        let metadata = JobMetadata { uid, state: JobState::NotStarted, result: None };

        self.jobs.write()
            .map_err(|x| StorageError::Unspecified(x.to_string()))?
            .insert(uid, metadata.clone());

        let serialized_job = serde_json::to_string(&job)?;
        self.job_queue.0.send((uid, serialized_job)).await
            .map_err(|x| StorageError::Unspecified(x.to_string()))?;

        Ok(metadata)
    }

    async fn set_job_result(
        &mut self,
        uid: Ulid,
        job_result: Result<(), JobRunError>,
    ) -> Result<JobMetadata, StorageError> {
        let mut jobs = self.jobs.write()
            .map_err(|x| StorageError::Unspecified(x.to_string()))?;

        let mut metadata = jobs.get_mut(&uid)
            .ok_or(StorageError::Unspecified("Uid not found".to_string()))?;

        metadata.state = if job_result.is_ok() {
            JobState::Completed
        } else {
            JobState::Failed
        };
        metadata.result = job_result.err();

        Ok(metadata.clone())
    }

    async fn get_job(&self, uid: Ulid) -> Result<JobMetadata, StorageError> {
        let jobs = self.jobs.read()
            .map_err(|x| StorageError::Unspecified(x.to_string()))?;
        let metadata = jobs.get(&uid)
            .ok_or(StorageError::Unspecified(format!("Uid not found: {}", uid)))?;
        Ok(metadata.clone())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(Box::new(err))
    }
}
