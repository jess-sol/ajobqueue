use std::error::Error;

use async_trait::async_trait;

use crate::{error::JobRunError, Job};

mod in_memory;

pub use in_memory::InMemoryStorageProvider;

// TODO - try to type erase like erased_serde
// This would allow StorageProvider to work for all Job types with a single instantiation
#[async_trait]
pub trait StorageProvider<J: Job + ?Sized>: Send + Sync {
    async fn create_job(&mut self, job: &J) -> Result<(), JobRunError>;
    async fn get_job(&mut self) -> Result<Box<J>, JobRunError>;
    async fn set_job_result(
        &mut self,
        result: Result<(), Box<dyn Error + Sync + Send>>,
    ) -> Result<(), JobRunError>;
}
