use async_trait::async_trait;
use ulid::Ulid;

use crate::{
    error::{JobRunError, StorageError},
    JobTypeMarker,
};

mod in_memory;

pub use in_memory::InMemoryStorageProvider;

pub enum JobState {
    NotStarted,
    Running,
    Completed,
    Failed
}

#[derive(Debug)]
pub struct JobMetadata {
    uid: Ulid,
    state: JobState,
    result: Option<JobRunError>,
}

// TODO - try to type erase like erased_serde
// This would allow StorageProvider to work for all Job types with a single instantiation
#[async_trait]
pub trait StorageProvider<J: JobTypeMarker + ?Sized>: Send + Sync {
    async fn push(&mut self, job: &J) -> Result<JobMetadata, StorageError>;
    async fn pull(&mut self) -> Result<Box<J>, StorageError>;
    async fn set_job_result(&mut self, result: Result<(), JobRunError>)
        -> Result<(), StorageError>;
}
