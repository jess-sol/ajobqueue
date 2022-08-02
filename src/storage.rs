use async_trait::async_trait;
use ulid::Ulid;

use crate::{
    error::{JobRunError, StorageError},
    JobTypeMarker,
};

mod in_memory;

#[cfg(feature="postgres")]
pub mod postgres;

pub use in_memory::InMemoryStorageProvider;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "postgres", derive(sqlx::Type))]
#[cfg_attr(feature = "postgres", sqlx(type_name = "job_state"))]
#[cfg_attr(feature = "postgres", sqlx(rename_all = "kebab-case"))]
pub enum JobState {
    NotStarted,
    Running,
    Completed,
    Failed
}

#[derive(Clone, Debug)]
pub struct JobMetadata {
    uid: Ulid,
    state: JobState,
    result: Option<JobRunError>,
}

#[derive(Clone, Debug)]
pub struct JobInfo<J: JobTypeMarker + ?Sized> {
    metadata: JobMetadata,
    job: Box<J>,
}

// TODO - try to type erase like erased_serde
// This would allow StorageProvider to work for all Job types with a single instantiation
#[async_trait]
pub trait StorageProvider<J: JobTypeMarker + ?Sized>: Send + Sync {
    async fn push(&mut self, job: &J) -> Result<JobMetadata, StorageError>;
    async fn pull(&mut self) -> Result<Box<J>, StorageError>;
    async fn set_job_result(&mut self, result: Result<(), JobRunError>)
        -> Result<(), StorageError>;
    async fn get_job(&self, job_id: Ulid) -> Result<JobInfo<J>, StorageError>;
}
