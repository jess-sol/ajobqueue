use std::any::Any;
use std::marker::PhantomData;

use async_trait::async_trait;

mod error;
mod executor;
mod storage;

pub use executor::Executor;
pub use storage::StorageProvider;

#[async_trait]
pub trait Job: Sync + Send {
    type JobTypeData: Any;
    async fn run(&self, job_data: &Self::JobTypeData);
}

pub struct Queue<J: Job + ?Sized> {
    _phantom_type: PhantomData<J>,
    storage_provider: Box<dyn StorageProvider<Job = J>>,
}

impl<J: Job + ?Sized> Queue<J> {
    fn new(storage_provider: Box<dyn StorageProvider<Job = J>>) -> Self {
        Queue {
            _phantom_type: PhantomData,
            storage_provider,
        }
    }

    async fn push_job(&mut self, job: Box<J>) -> Result<(), ()> {
        self.storage_provider.create_job(job).await.unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{storage::InMemoryStorageProvider, Executor, Job, Queue};
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
        let mut queue = Queue::new(Box::new(storage_provider.clone()));

        let job = MockJob {
            msg: "world!".to_string(),
        };
        queue.push_job(Box::new(job)).await.unwrap();

        let mut executor = Executor::new(
            Box::new(storage_provider),
            MockJobType {
                data_msg_type: "Hello".to_string(),
            },
        );

        executor.start().await;
    }
}
