use std::error::Error;

use async_trait::async_trait;

use crate::{Job, error::JobRunError};

pub use in_memory::InMemoryStorageProvider;

// TODO - try to type erase like erased_serde
// This would allow StorageProvider to work for all Job types with a single instantiation
#[async_trait]
pub trait StorageProvider: Send + Sync {
    type Job: Job + ?Sized;
    async fn create_job(&mut self, job: Box<Self::Job>) -> Result<(), JobRunError>;
    async fn get_job(&mut self) -> Result<Box<Self::Job>, JobRunError>;
    async fn set_job_result(&mut self, result: Result<(), Box<dyn Error+Sync+Send>>) -> Result<(), JobRunError>;
}

mod in_memory {
    use std::{sync::Arc, error::Error, marker::PhantomData};
    use std::fmt::Debug;

    use async_trait::async_trait;
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    use tokio::sync::Mutex;

    use super::StorageProvider;
    use crate::{Job, error::JobRunError};

    pub struct InMemoryStorageProvider<J: Job + ?Sized> {
        // jobs: Vec<Box<<Self as StorageProvider>::Job>>,
        jobs: Arc<Mutex<Vec<String>>>,
        _phantom_type: PhantomData<J>
    }

    impl<J: Job + ?Sized> Clone for InMemoryStorageProvider<J> {
        fn clone(&self) -> Self {
            Self {
                jobs: self.jobs.clone(),
                _phantom_type: PhantomData,
            }
        }
    }

    impl<J: Job + ?Sized> InMemoryStorageProvider<J> {
        pub fn new() -> Self {
            InMemoryStorageProvider {
                jobs: Arc::new(Mutex::new(Vec::with_capacity(10))),
                _phantom_type: PhantomData
            }
        }
    }


    #[async_trait]
    // TODO - Generic phantom type only alternative until GATs or similar is GA
    impl<J: Job + Debug + Serialize + ?Sized> StorageProvider for InMemoryStorageProvider<J>
        where Box<J>: DeserializeOwned
    {
        type Job = J;
        async fn get_job(&mut self) -> Result<Box<Self::Job>, JobRunError> {
            let serialized_job = self.jobs.lock().await.pop().unwrap();
            let job: Box<Self::Job> = serde_json::from_str(&serialized_job).unwrap();
            Ok(job)
        }

        async fn create_job(&mut self, job: Box<Self::Job>) -> Result<(), JobRunError> {
            let serialized_job = serde_json::to_string(&job).unwrap();
            self.jobs.lock().await.push(serialized_job);
            Ok(())
        }

        async fn set_job_result(&mut self, _result: Result<(), Box<dyn Error+Sync+Send>>) -> Result<(), JobRunError> {
            Err(JobRunError {})
        }
    }
}
