use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;

use async_trait::async_trait;

mod error;
mod executor;
mod storage;

pub use executor::Executor;
pub use storage::StorageProvider;

#[async_trait]
pub trait Job: erased_serde::Serialize + Debug + Sync + Send {
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
    use serde::{Deserialize, Serialize};

    // JobType macro {{{
    // #[job_type]
    struct MockJobTypeData {
        data_msg_type: String,
    }

    #[async_trait]
    #[typetag::serde(tag = "type")]
    trait MockJobType: Job<JobTypeData = MockJobTypeData> {}
    // }}}

    // Job {{{
    // #[job(MockJobType)]
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct MockJob {
        msg: String,
    }

    #[typetag::serde]
    impl MockJobType for MockJob {}

    #[async_trait]
    impl Job for MockJob {
        type JobTypeData = MockJobTypeData;

        async fn run(&self, job_data: &Self::JobTypeData) {
            println!("MSG: {}, {}", job_data.data_msg_type, self.msg);
        }
    }
    // }}}

    // JobType macro {{{
    // #[job_type]
    struct OtherJobTypeData {
        data_msg_type: String,
    }

    #[async_trait]
    #[typetag::serde(tag = "type")]
    trait OtherJobType: Job<JobTypeData = OtherJobTypeData> {}
    // }}}

    // Job {{{
    // #[job(OtherJobType)]
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct OtherJob {
        msg: String,
    }

    #[typetag::serde]
    impl OtherJobType for OtherJob {}

    #[async_trait]
    impl Job for OtherJob {
        type JobTypeData = OtherJobTypeData;

        async fn run(&self, job_data: &Self::JobTypeData) {
            println!("MSG: {}, {}", job_data.data_msg_type, self.msg);
        }
    }
    // }}}

    #[tokio::test]
    async fn it_works() {
        let storage_provider = InMemoryStorageProvider::<dyn MockJobType>::new();
        let mut queue = Queue::new(Box::new(storage_provider.clone()));

        let job = MockJob {
            msg: "world!".to_string(),
        };
        queue.push_job(Box::new(job)).await.unwrap();

        let mut executor = Executor::new(
            Box::new(storage_provider),
            MockJobTypeData {
                data_msg_type: "Hello".to_string(),
            },
        );

        executor.start().await;
    }
}
