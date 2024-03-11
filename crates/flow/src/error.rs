use crate::flow_graph::BuildGraphError;
use flow_lib::{command::CommandError, Name};
use std::error::Error as StdError;
use thiserror::Error as ThisError;

pub type BoxedError = Box<dyn StdError + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

fn unwrap(s: &Option<String>) -> &str {
    s.as_ref().map(|v| v.as_str()).unwrap_or_default()
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    Any(#[from] BoxedError),
    #[error("canceled by user {}", unwrap(.0))]
    Canceled(Option<String>),
    #[error("value not found in field \"{0}\"")]
    ValueNotFound(Name),
    #[error("failed to create command: {}", .0)]
    CreateCmd(#[source] CommandError),
    #[error(transparent)]
    BuildGraphError(#[from] BuildGraphError),
}

impl Error {
    pub fn custom<E: Into<BoxedError>>(e: E) -> Self {
        Error::Any(e.into())
    }
}
