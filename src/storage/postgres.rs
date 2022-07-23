use std::marker::PhantomData;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::{
    error::{JobRunError, StorageError},
    JobTypeMarker, StorageProvider,
};

use super::JobMetadata;

pub struct PostgresStorageProvider<J: JobTypeMarker + ?Sized> {
    _phantom_data: PhantomData<J>,
}

impl<J: JobTypeMarker + ?Sized> PostgresStorageProvider<J> {
    fn new() -> Self {
        Self {
            _phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<J: JobTypeMarker + ?Sized> StorageProvider<J> for PostgresStorageProvider<J>
where
    Box<J>: DeserializeOwned,
{
    async fn pull(&mut self) -> Result<Box<J>, StorageError> {
        unimplemented!()
    }

    async fn push(&mut self, job: &J) -> Result<JobMetadata, StorageError> {
        unimplemented!()
    }

    async fn set_job_result(
        &mut self,
        _result: Result<(), JobRunError>,
    ) -> Result<(), StorageError> {
        unimplemented!()
    }
}
