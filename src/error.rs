use std::{error::Error as StdError, fmt};

use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum AJobQueueError {
    #[error("Execution error")]
    Execution(#[source] ExecutionError),

    #[error("Storage error")]
    Storage(#[source] StorageError),

    #[error("Job run error")]
    JobRun(#[source] JobRunError),
}

impl From<ExecutionError> for AJobQueueError {
    fn from(err: ExecutionError) -> Self {
        AJobQueueError::Execution(err)
    }
}

impl From<StorageError> for AJobQueueError {
    fn from(err: StorageError) -> Self {
        AJobQueueError::Storage(err)
    }
}

impl From<JobRunError> for AJobQueueError {
    fn from(err: JobRunError) -> Self {
        AJobQueueError::JobRun(err)
    }
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Failed to signal executors")]
    SignalingError(#[source] Box<dyn StdError + Send>),

    #[error("Failed to join executor tasks to main task")]
    JoinError(#[source] JoinError),
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Failed to fetch task")]
    FetchFailure(#[source] Box<dyn StdError + Send>),

    #[error("Failed to create task")]
    CreateFailure(#[source] Box<dyn StdError + Send>),

    #[error("Failed to create task")]
    SerializationError(#[source] Box<dyn StdError + Send>),

    #[error("Unspecified error")]
    UnspecifiedError(#[source] Box<dyn StdError + Send>),
}

#[derive(Error, Debug)]
pub enum JobRunError {
    #[error("Task failure")]
    TaskFailure(#[source] Box<dyn StdError + Send>),
}
