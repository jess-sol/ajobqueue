use std::error::Error as StdError;

use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum AJobQueueError {
    #[error("Execution error")]
    Execution(#[from] ExecutionError),

    #[error("Storage error")]
    Storage(#[from] StorageError),

    #[error("Job run error")]
    JobRun(#[from] JobRunError),
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Failed to signal executors")]
    SignalingError(#[source] Box<dyn StdError + Send>),

    #[error("Failed to join executor tasks to main task")]
    JoinError(#[from] JoinError),
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
