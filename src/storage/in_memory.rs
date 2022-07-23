use std::marker::PhantomData;
use std::error::Error;

use async_trait::async_trait;
use async_channel::{unbounded, Receiver, Sender};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::StorageProvider;
use crate::{error::JobRunError, Job};

// PhantomData necessary so struct only impls one generic impl of StorageProvider
pub struct InMemoryStorageProvider<J: ?Sized> {
    jobs: (Sender<String>, Receiver<String>),
    _phantom_data: PhantomData<J>,
}

impl<J: ?Sized> Clone for InMemoryStorageProvider<J> {
    fn clone(&self) -> Self {
        Self {
            jobs: self.jobs.clone(),
            _phantom_data: PhantomData,
        }
    }
}

impl<J: ?Sized> InMemoryStorageProvider<J> {
    pub fn new() -> Self {
        InMemoryStorageProvider {
            jobs: unbounded(),
            _phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<J: Job + ?Sized> StorageProvider<J> for InMemoryStorageProvider<J>
where
    Box<J>: DeserializeOwned,
    for<'a> &'a J: Serialize,
{
    async fn get_job(&mut self) -> Result<Box<J>, JobRunError> {
        let serialized_job = self.jobs.1.recv().await.map_err(|_| JobRunError {})?;
        let job: Box<J> = serde_json::from_str(&serialized_job).map_err(|_| JobRunError {})?;
        Ok(job)
    }

    async fn create_job(&mut self, job: &J) -> Result<(), JobRunError> {
        let serialized_job = serde_json::to_string(&job).unwrap();
        self.jobs.0.send(serialized_job).await.map_err(|_| JobRunError {})?;
        Ok(())
    }

    async fn set_job_result(
        &mut self,
        _result: Result<(), Box<dyn Error + Sync + Send>>,
    ) -> Result<(), JobRunError> {
        Err(JobRunError {})
    }
}
