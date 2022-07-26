use std::marker::PhantomData;

use async_channel::{unbounded, Receiver, Sender};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use ulid::Ulid;

use super::{StorageProvider, JobMetadata, JobState};
use crate::{
    error::{JobRunError, StorageError},
    JobTypeMarker,
};

// PhantomData necessary so struct only impls one generic impl of StorageProvider
pub struct InMemoryStorageProvider<J: JobTypeMarker + ?Sized> {
    jobs: (Sender<String>, Receiver<String>),
    _phantom_data: PhantomData<J>,
}

impl<J: JobTypeMarker + ?Sized> Clone for InMemoryStorageProvider<J> {
    fn clone(&self) -> Self {
        Self {
            jobs: self.jobs.clone(),
            _phantom_data: PhantomData,
        }
    }
}

impl<J: JobTypeMarker + ?Sized> InMemoryStorageProvider<J> {
    pub fn new() -> Self {
        InMemoryStorageProvider {
            jobs: unbounded(),
            _phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<J: JobTypeMarker + ?Sized> StorageProvider<J> for InMemoryStorageProvider<J>
where Box<J>: DeserializeOwned
{
    async fn pull(&mut self) -> Result<Box<J>, StorageError> {
        let serialized_job = self.jobs.1.recv().await
            .map_err(|x| StorageError::FetchFailure(Box::new(x)))?;
        let job: Box<J> = serde_json::from_str(&serialized_job)?;
        Ok(job)
    }

    async fn push(&mut self, job: &J) -> Result<JobMetadata, StorageError> {
        let serialized_job = serde_json::to_string(&job)?;
        self.jobs.0.send(serialized_job).await
            .map_err(|x| StorageError::CreateFailure(Box::new(x)))?;
        Ok(JobMetadata {
            uid: Ulid::new(),
            state: JobState::NotStarted,
            result: None,
        })
    }

    async fn set_job_result(
        &mut self,
        _result: Result<(), JobRunError>,
    ) -> Result<(), StorageError> {
        unimplemented!()
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(Box::new(err))
    }
}
