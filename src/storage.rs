use async_trait::async_trait;

use crate::{
    error::{JobRunError, StorageError},
    JobTypeMarker,
};

mod in_memory;

pub use in_memory::InMemoryStorageProvider;

// TODO - try to type erase like erased_serde
// This would allow StorageProvider to work for all Job types with a single instantiation
#[async_trait]
pub trait StorageProvider<J: JobTypeMarker + ?Sized>: Send + Sync {
    async fn create_job(&mut self, job: &J) -> Result<(), StorageError>;
    async fn get_job(&mut self) -> Result<Box<J>, StorageError>;
    async fn set_job_result(&mut self, result: Result<(), JobRunError>)
        -> Result<(), StorageError>;
}
