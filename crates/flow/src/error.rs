use crate::{
    flow_graph::BuildGraphError,
    flow_registry::get_flow,
    flow_set::{get_flow_row, make_signer},
};
use flow_lib::{Name, command::CommandError};
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
    #[error(transparent)]
    GetFlow(#[from] get_flow::Error),
    #[error(transparent)]
    GetFlowRow(#[from] get_flow_row::Error),
    #[error(transparent)]
    MakeSigner(#[from] make_signer::Error),
    #[error("graph has cycle")]
    Cycle,
    #[error("flow must contain exactly 1 tx")]
    NeedOneTx,
    #[error("flow must have exactly 1 Flow Output node connected to a signature output")]
    NeedOneSignatureOutput,
}

impl Error {
    pub fn custom<E: Into<BoxedError>>(e: E) -> Self {
        Error::Any(e.into())
    }
}
