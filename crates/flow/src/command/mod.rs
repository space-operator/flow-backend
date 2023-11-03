use flow_lib::command::CommandError;

pub mod collect;
pub mod flow_input;
pub mod flow_output;
pub mod foreach;
pub mod interflow;
pub mod interflow_instructions;
pub mod wasm;

pub mod prelude {
    pub use async_trait::async_trait;
    pub use flow_lib::{
        command::{
            builder::{BuildResult, BuilderCache, CmdBuilder},
            CommandDescription, CommandError, CommandTrait,
        },
        config::client::NodeData,
        context::Context,
        CmdInputDescription as Input, CmdOutputDescription as Output, FlowId, Name, Value,
        ValueSet, ValueType,
    };
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value as JsonValue;
    pub use std::sync::Arc;
    pub use thiserror::Error as ThisError;
}

#[derive(serde::Deserialize)]
pub struct ErrorBody {
    pub error: String,
}

pub async fn supabase_error(code: reqwest::StatusCode, resp: reqwest::Response) -> CommandError {
    let bytes = resp.bytes().await.unwrap_or_default();
    match serde_json::from_slice::<ErrorBody>(&bytes) {
        Ok(ErrorBody { error }) => CommandError::msg(error),
        _ => {
            let body = String::from_utf8_lossy(&bytes);
            anyhow::anyhow!("{}: {}", code, body)
        }
    }
}
