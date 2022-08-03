use async_trait::async_trait;
use ulid::Ulid;

use crate::{
    error::{JobRunError, StorageError},
    JobTypeMarker,
};

mod in_memory;

#[cfg(feature="postgres")]
pub mod postgres;
#[cfg(feature="postgres")]
pub use postgres::PostgresStorageProvider;

pub use in_memory::InMemoryStorageProvider;

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub uid: Ulid,
    pub state: JobState,
    pub result: Option<JobRunError>,
}

#[derive(Clone, Debug)]
pub struct JobInfo<J: JobTypeMarker + ?Sized> {
    pub metadata: JobMetadata,
    pub job: Box<J>,
}

// TODO - try to type erase like erased_serde
// This would allow StorageProvider to work for all Job types with a single instantiation
#[async_trait]
pub trait StorageProvider<J: JobTypeMarker + ?Sized>: Send + Sync {
    async fn push(&mut self, job: &J) -> Result<JobMetadata, StorageError>;
    async fn pull(&mut self) -> Result<JobInfo<J>, StorageError>;
    async fn set_job_result(&mut self, uid: Ulid, job_result: Result<(), JobRunError>)
        -> Result<JobMetadata, StorageError>;
    async fn get_job(&self, job_id: Ulid) -> Result<JobMetadata, StorageError>;
}
