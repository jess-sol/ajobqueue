extern crate self as ajobqueue;

use std::fmt::Debug;

use async_trait::async_trait;
use serde::Serialize;
use storage::JobInfo;
use ulid::Ulid;

mod error;
mod executor;
pub mod storage;

pub use ajobqueue_macro::*;
pub use error::AJobQueueError;
pub use executor::Executor;
pub use storage::StorageProvider;

#[doc(hidden)]
pub use typetag;

#[async_trait]
pub trait Job: Sync + Send + Debug {
    type JobTypeData: JobType;
    async fn run(&self, job_data: &Self::JobTypeData);
}

pub trait JobTypeMarker: Job + Serialize {}

pub trait JobType: Send + Sync {
    fn job_type() -> String;
}

impl<J: ?Sized, T> JobType for J
where
    J: Job<JobTypeData = T>,
    T: JobType,
{
    fn job_type() -> String {
        T::job_type()
    }
}

pub struct Queue<J: JobTypeMarker + ?Sized> {
    storage_provider: Box<dyn StorageProvider<J>>,
}

impl<J: JobTypeMarker + ?Sized> Queue<J> {
    pub fn new<S: StorageProvider<J> + 'static>(storage_provider: S) -> Self {
        Queue {
            storage_provider: Box::new(storage_provider),
        }
    }

    pub async fn push_job(&mut self, job: &J) -> Result<(), AJobQueueError> {
        self.storage_provider.push(job).await?;
        Ok(())
    }

    pub async fn get_job(&self, job_uid: Ulid) -> Result<JobInfo<J>, AJobQueueError> {
        let result = self.storage_provider.get_job(job_uid).await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use crate::{job, job_type, storage::InMemoryStorageProvider, Executor, Job, Queue};
    use async_trait::async_trait;

    // Job type 1
    #[job_type]
    struct MockJobType {
        data_msg_type: String,
        shared_data: Arc<Mutex<Vec<String>>>,
    }

    #[job(MockJobType)]
    struct MockJob {
        msg: String,
    }

    #[async_trait]
    impl Job for MockJob {
        type JobTypeData = MockJobType;

        async fn run(&self, job_data: &Self::JobTypeData) {
            let msg = format!("MSG: {}, {}", job_data.data_msg_type, self.msg);
            job_data.shared_data.lock().await.push(msg);
        }
    }

    #[job(MockJobType)]
    struct MockJob2 {
        msg: String,
    }

    #[async_trait]
    impl Job for MockJob2 {
        type JobTypeData = MockJobType;

        async fn run(&self, job_data: &Self::JobTypeData) {
            let msg = format!("MSG2: {}, {}", job_data.data_msg_type, self.msg);
            job_data.shared_data.lock().await.push(msg);
        }
    }

    // Job type 2
    #[job_type]
    struct OtherJobType {}

    #[job(OtherJobType)]
    struct OtherJob {
        msg: String,
    }

    #[async_trait]
    impl Job for OtherJob {
        type JobTypeData = OtherJobType;
        async fn run(&self, _: &Self::JobTypeData) {}
    }

    #[tokio::test]
    async fn it_works() {
        let _ = env_logger::builder().is_test(true).try_init();

        let storage_provider = InMemoryStorageProvider::<dyn MockJobTypeMarker>::new();
        let mut queue = Queue::new(storage_provider.clone());

        queue.push_job(&MockJob { msg: "world!".to_string() }).await.unwrap();
        queue.push_job(&MockJob2 { msg: "world!".to_string() }).await.unwrap();

        let shared_data = Arc::new(Mutex::new(Vec::new()));

        let executor = Executor::new(
            storage_provider,
            MockJobType {
                data_msg_type: "Hello".to_string(),
                shared_data: shared_data.clone(),
            },
        );

        let executor = executor.start().await;

        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        executor.stop().await.unwrap();

        assert_eq!(*shared_data.lock().await, vec![
            "MSG: Hello, world!",
            "MSG2: Hello, world!"
        ]);
    }
}
