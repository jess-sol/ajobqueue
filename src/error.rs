use std::{error::Error, fmt};

#[derive(Debug)]
pub struct JobRunError {}

impl fmt::Display for JobRunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for JobRunError {}
