use std::error::Error;

use async_trait::async_trait;

use crate::{error::JobRunError, Job};

pub use in_memory::InMemoryStorageProvider;

// TODO - try to type erase like erased_serde
// This would allow StorageProvider to work for all Job types with a single instantiation
#[async_trait]
pub trait StorageProvider<J: Job + ?Sized>: Send + Sync {
    async fn create_job(&mut self, job: &J) -> Result<(), JobRunError>;
    async fn get_job(&mut self) -> Result<Box<J>, JobRunError>;
    async fn set_job_result(
        &mut self,
        result: Result<(), Box<dyn Error + Sync + Send>>,
    ) -> Result<(), JobRunError>;
}

mod in_memory {
    use std::marker::PhantomData;
    use std::{error::Error, sync::Arc};

    use async_trait::async_trait;
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    use tokio::sync::Mutex;

    use super::StorageProvider;
    use crate::{error::JobRunError, Job};

    // PhantomData necessary so struct only impls one generic impl of StorageProvider
    pub struct InMemoryStorageProvider<J: ?Sized> {
        jobs: Arc<Mutex<Vec<String>>>,
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
                jobs: Arc::new(Mutex::new(Vec::with_capacity(10))),
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
            let serialized_job = self.jobs.lock().await.pop().unwrap();
            let job: Box<J> = serde_json::from_str(&serialized_job).unwrap();
            Ok(job)
        }

        async fn create_job(&mut self, job: &J) -> Result<(), JobRunError> {
            let serialized_job = serde_json::to_string(&job).unwrap();
            self.jobs.lock().await.push(serialized_job);
            Ok(())
        }

        async fn set_job_result(
            &mut self,
            _result: Result<(), Box<dyn Error + Sync + Send>>,
        ) -> Result<(), JobRunError> {
            Err(JobRunError {})
        }
    }
}
