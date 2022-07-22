use std::{error::Error, any::Any};

use async_trait::async_trait;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use erased_serde::{Serializer, Deserializer};

pub trait JobFamily {
    // String representation of JobFamily. Must be unique across all JobFamily's used with a single
    // `Storage` backend.
    fn job_family_name() -> String;
}

pub trait Executor where Self: Sync {
    type JobFamily: JobFamily;

    fn new(worker_data: Self::JobFamily) -> Self where Self: Sized;
    fn worker_data(&self) -> &Self::JobFamily;
}

pub trait Storage {
    fn create_job<J: Job + Serialize>(&mut self, job: &J) -> Result<(), ()>;
    fn get_job<'a, J: Job + DeserializeOwned>(&self) -> Result<(), ()>;
    fn set_job_result<JF: JobFamily>(&mut self, result: Result<(), Box<dyn Error>>) -> Result<(), ()>;
}

#[async_trait]
pub trait Job where Self: Sync + erased_serde::Serialize {
    type JobFamily: Any + Send;

    async fn run(&self, executor: &dyn Executor<JobFamily=Self::JobFamily>) -> Result<(), Box<dyn Error>>;
}

pub struct JobQueue<E: Executor> {
    executor: E,
}

impl<E: Executor> JobQueue<E> {
    pub fn new(executor: E) -> Self {
        Self {
            executor
        }
    }

    // pub async fn start() {
    // }
}


// InMemoryStorage start
use std::collections::HashMap;

struct InMemoryStorage {
    jobs: HashMap<String, Vec<Vec<u8>>>,
}

impl InMemoryStorage {
    fn new() -> Self {
        InMemoryStorage {
            jobs: HashMap::new(),
        }
    }
}

impl Storage for InMemoryStorage {
    fn create_job<J: Job + Serialize>(&mut self, job: &J) -> Result<(), ()> {
        let mut output = Vec::with_capacity(128);
        let json = &mut serde_json::Serializer::new(&mut output);
        let mut z = Box::new(<dyn Serializer>::erase(json));
        job.erased_serialize(&mut *z).unwrap();

        self.jobs
            .entry("test".to_owned())
            // .entry(<J as Job>::JobFamily::job_family_name())
            .or_insert(Vec::new())
            .push(output);

        // println!("HIDER: {:?}", output);

        // let json = &mut serde_json::Deserializer::from_slice(&output);
        // let json = Box::new(<dyn Deserializer>::erase(json));
        // let data: J = erased_serde::deserialize(&mut *json).unwrap();
        Ok(())
    }

    fn get_job<'a, J: Job + DeserializeOwned>(&self) -> Result<(), ()> {
        Ok(())
    }

    fn set_job_result<JF: JobFamily>(&mut self, result: Result<(), Box<dyn Error>>) -> Result<(), ()> {
        Ok(())
    }
}


// InMemoryStorage end

#[cfg(test)]
mod tests {
    use std::error::Error;
    use async_trait::async_trait;
    use linkme::distributed_slice;

    use crate::InMemoryStorage;

    use super::{JobFamily, Job, Executor, Storage};

    use ajobqueue_macro::executor;

    macro_rules! ty {($type:ty) => {std::any::type_name::<$type>()}}

    struct MsgJobFamily {
        msg: String,
    }

    impl JobFamily for MsgJobFamily {
        fn job_family_name() -> String {
            String::from("msg")
        }
    }

    #[executor]
    struct MockExecutor {
        worker_data: MsgJobFamily,
    }

    impl Executor for MockExecutor {
        type JobFamily = MsgJobFamily;

        fn new(worker_data: Self::JobFamily) -> Self where Self: Sized {
            Self { worker_data }
        }
        fn worker_data(&self) -> &Self::JobFamily {
            &self.worker_data
        }
    }

    // Executor macro
    // #[ajobqueue::executor]
    // impl Executor for MockExecutor { .. }
    // use MockExecutorImpl::MockExecutor;
    // #[allow(non_snake_case)]
    // mod MockExecutorImpl {
    //     // TODO - Use ajobqueue instead of crate
    //     use crate::{Job, Executor};
    //     use linkme::distributed_slice;

    //     type JobFamily = super::MsgJobFamily;

    //     #[distributed_slice]
    //     pub(super) static JOBS: [fn() -> Box<dyn Job<JobFamily=JobFamily>>] = [..];

    //     pub(super) struct MockExecutor {
    //         pub(super) worker_data: JobFamily,
    //     }

    //     impl Executor for MockExecutor {
    //         type JobFamily = JobFamily;

    //         fn new(worker_data: Self::JobFamily) -> Self {
    //             let x = JOBS;
    //             println!("HIDER: {}", ty!(JOBS));
    //             // println!("HIDER: {:?}", JOBS!());
    //             Self { worker_data }
    //         }

    //         fn worker_data(&self) -> &Self::JobFamily {
    //             &self.worker_data
    //         }
    //     }
    // }
    // End executor macro

    // Job macro
    #[derive(serde::Serialize)]
    struct MockJob {}

    #[distributed_slice(MockExecutorImpl::JOBS)]
    fn mock_job_generator() -> Box<dyn Job<JobFamily=MsgJobFamily>> {
        Box::new(MockJob {})
    }

    // #[ajobqueue::job]
    // impl Job for MockJob { .. }
    #[async_trait]
    impl Job for MockJob {
        type JobFamily = MsgJobFamily;
        async fn run(&self, executor: &dyn Executor<JobFamily=Self::JobFamily>) -> Result<(), Box<dyn Error>> {
            println!("Msg: {}", executor.worker_data().msg);
            Ok(())
        }
    }
    // End job macro

    #[tokio::test]
    async fn it_works() {
        let executor = MockExecutor::new(MsgJobFamily { msg: "test".to_owned() });
        let job = MockJob {};

        job.run(&executor as _).await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_storage() {
        let mut storage = InMemoryStorage::new();
        storage.create_job(&MockJob {}).unwrap();
    }
}
