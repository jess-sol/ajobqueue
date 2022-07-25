use std::marker::PhantomData;

use async_trait::async_trait;

mod error;
mod executor;
mod storage;

use error::AJobQueueError;
pub use executor::Executor;
pub use storage::StorageProvider;

#[async_trait]
pub trait Job: Sync + Send {
    type JobTypeData: JobType;
    async fn run(&self, job_data: &Self::JobTypeData);
}

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

pub struct Queue<J: Job + ?Sized> {
    _phantom_type: PhantomData<J>,
    storage_provider: Box<dyn StorageProvider<J>>,
}

impl<J: Job + ?Sized> Queue<J> {
    pub fn new<S: StorageProvider<J> + 'static>(storage_provider: S) -> Self {
        Queue {
            _phantom_type: PhantomData,
            storage_provider: Box::new(storage_provider),
        }
    }

    pub async fn push_job(&mut self, job: &J) -> Result<(), AJobQueueError> {
        self.storage_provider.create_job(job).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{storage::InMemoryStorageProvider, Executor, Job, JobType, Queue};
    use async_trait::async_trait;

    use ajobqueue_macro::{job, job_type};

    // Job type 1
    #[job_type]
    struct MockJobType {
        data_msg_type: String,
    }

    #[job(MockJobType)]
    struct MockJob {
        msg: String,
    }

    #[async_trait]
    impl Job for MockJob {
        type JobTypeData = MockJobType;

        async fn run(&self, job_data: &Self::JobTypeData) {
            println!("MSG: {}, {}", job_data.data_msg_type, self.msg);
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
            println!("MSG@: {}, {}", job_data.data_msg_type, self.msg);
        }
    }

    // Job type 2
    #[job_type]
    struct OtherJobType {
        data_msg_type: String,
    }

    #[job(OtherJobType)]
    struct OtherJob {
        msg: String,
    }

    #[async_trait]
    impl Job for OtherJob {
        type JobTypeData = OtherJobType;

        async fn run(&self, job_data: &Self::JobTypeData) {
            println!("MSG: {}, {}", job_data.data_msg_type, self.msg);
        }
    }

    #[tokio::test]
    async fn it_works() {
        let storage_provider = InMemoryStorageProvider::new();
        let mut queue = Queue::new(storage_provider.clone());

        queue.push_job(&MockJob { msg: "world!".to_string() }).await.unwrap();
        queue.push_job(&MockJob2 { msg: "world!".to_string() }).await.unwrap();

        let executor = Executor::new(
            storage_provider,
            MockJobType {
                data_msg_type: "Hello".to_string(),
            },
        );

        let executor = executor.start().await;

        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        executor.stop().await.unwrap();
    }
}
