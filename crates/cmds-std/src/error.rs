use std::error::Error as StdError;
use std::result::Result as StdResult;
use thiserror::Error as ThisError;

pub type BoxedError = Box<dyn StdError + Send + Sync>;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Any(#[from] anyhow::Error),

    #[error(transparent)]
    Value(#[from] value::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("worker stopped")]
    WorkerStopped,
    #[error("time-out waiting for signature")]
    SignatureTimeout,
    #[error("an error occured while running rhai expression: {0}")]
    RhaiExecutionError(String),
    #[error("value not found in field \"{0}\"")]
    ValueNotFound(String),
}

impl Error {
    pub fn custom<E: Into<anyhow::Error>>(e: E) -> Self {
        Error::Any(e.into())
    }
}
