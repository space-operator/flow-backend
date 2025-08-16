//! Utilities to use within nodes and flows.
//!
//! Table of contents:
//! - [`command`]: implementing a new command.
//! - [`config`]: types definition
//! - [`context`]: providing services and information for nodes to use.
//! - [`solana`]: utilities for working with Solana.
//! - [`utils`]: other utilities.

pub mod command;
pub mod config;
pub mod context;
pub mod flow_run_events;
pub mod utils;
pub mod solana;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub type UserId = uuid::Uuid;

pub use config::{
    CmdInputDescription, CmdOutputDescription, CommandType, ContextConfig, FlowConfig, FlowId,
    FlowRunId, Gate, HttpClientConfig, Name, NodeConfig, NodeId, SolanaClientConfig, SolanaNet,
    ValueSet, ValueType,
};
pub use context::User;
pub use inventory::submit;
pub use value::{self, Error as ValueError, Value};

/// Helper macro to read node definition file at compile-time.
///
/// `node_definition!("node.json")` will expand to read file at
/// `$CARGO_MANIFEST_DIR/node-definitions/node.json`.
///
/// See: [CARGO_MANIFEST_DIR](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
#[macro_export]
macro_rules! node_definition {
    ($file:expr $(,)?) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/node-definitions/",
            $file
        ))
    };
}
