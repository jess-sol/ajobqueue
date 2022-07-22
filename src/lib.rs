use std::{sync::Arc, error::Error, marker::PhantomData};
use std::fmt::{self, Debug};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};

use tokio::sync::Mutex;

#[async_trait]
pub trait Executor: Send + Sync {
    async fn start(&mut self);
}

#[async_trait]
pub trait Job: erased_serde::Serialize + Debug + Sync + Send {
    type Executor: Executor;
    async fn run(&self, executor: &Self::Executor);
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    type Job: Job + ?Sized;
    async fn create_job(&mut self, job: Box<Self::Job>) -> Result<(), JobRunError>;
    async fn get_job(&mut self) -> Result<Box<Self::Job>, JobRunError>;
    async fn set_job_result(&mut self, result: Result<(), Box<dyn Error+Sync+Send>>) -> Result<(), JobRunError>;
}


#[derive(Debug)]
pub struct JobRunError {}

impl fmt::Display for JobRunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for JobRunError {}

pub struct Queue<J: Job + ?Sized> {
    _phantom_type: PhantomData<J>,
    storage_provider: Box<dyn StorageProvider<Job=J>>,
}

impl<J: Job + ?Sized> Queue<J> {
    fn new(storage_provider: Box<dyn StorageProvider<Job=J>>) -> Self {
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

struct InMemoryStorageProvider<J: Job + ?Sized> {
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
    fn new() -> Self {
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

    async fn set_job_result(&mut self, result: Result<(), Box<dyn Error+Sync+Send>>) -> Result<(), JobRunError> {
        Err(JobRunError {})
    }
}


// Executor macro {{{
// #[executor]
struct MockExecutor {
    // TODO - Monomorphize storage_provider? Removes vtable lookup
    storage_provider: Box<dyn StorageProvider<Job=dyn MockExecutorJob>>,
    data_msg_type: String,
}

// TODO - Manual monomorphization with macro?
#[async_trait]
impl Executor for MockExecutor {
    async fn start(&mut self) {
        let job: Box<dyn MockExecutorJob> = self.storage_provider.get_job().await.unwrap();
        Job::run(&*job, self).await;
    }
}

#[async_trait]
#[typetag::serde(tag = "type")]
trait MockExecutorJob: Job<Executor=MockExecutor> { }
// }}}

// Job {{{
// #[job(MockExecutor)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct MockJob {
    msg: String,
}

#[typetag::serde]
impl MockExecutorJob for MockJob { }

#[async_trait]
impl Job for MockJob {
    type Executor = MockExecutor;

    async fn run(&self, executor: &Self::Executor) {
        println!("MSG: {}, {}", executor.data_msg_type, self.msg);
    }
}
// }}}


#[cfg(test)]
mod tests {
    use super::Executor;
    use super::Queue;

    use super::MockJob;
    use super::MockExecutor;
    use super::MockExecutorJob;
    use super::InMemoryStorageProvider;

    #[tokio::test]
    async fn it_works() {
        let storage_provider = InMemoryStorageProvider::<dyn MockExecutorJob>::new();
        let mut queue = Queue::new(Box::new(storage_provider.clone()));

        let job = MockJob { msg: "world!".to_string() };
        queue.push_job(Box::new(job)).await.unwrap();

        let mut executor = MockExecutor {
            storage_provider: Box::new(storage_provider),
            data_msg_type: "Hello".to_string(),
        };
        executor.start().await;
    }
}
