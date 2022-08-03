use std::error::Error as StdError;

use serde::{Serialize, Deserialize};
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
    #[error("Failed to signal executors: {0:?}")]
    SignalingError(#[source] Box<dyn StdError + Send + Sync>),

    #[error("Failed to join executor tasks to main task: {0:?}")]
    JoinError(#[from] JoinError),
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("Failed to create task")]
    Serialization(#[source] Box<dyn StdError + Send + Sync>),

    #[error("Unspecified error: {0}")]
    Unspecified(String),
}

#[derive(Error, Clone, Debug, Serialize, Deserialize)]
pub enum JobRunError {
    #[error("Task failure")]
    TaskFailure {
        msg: String,
    },
}
