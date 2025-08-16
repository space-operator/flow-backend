use flow_lib::command::CommandError;

pub mod const_cmd;
pub mod error;
pub mod flow_run_info;
pub mod http_request;
pub mod json_extract;
pub mod json_insert;
pub mod kvstore;
pub mod note;
pub mod postgrest;
pub mod print_cmd;
pub mod std;
pub mod storage;
pub mod supabase;
pub mod wait_cmd;

pub mod prelude {
    pub use async_trait::async_trait;
    pub use flow_lib::{
        CmdInputDescription as CmdInput, CmdOutputDescription as CmdOutput, Name, SolanaNet,
        ValueSet, ValueType,
        command::{
            CommandDescription, CommandError, CommandTrait, InstructionInfo,
            builder::{BuildResult, BuilderCache, BuilderError, CmdBuilder},
        },
        context::CommandContext,
        solana::Instructions,
    };
    pub use rust_decimal::Decimal;
    pub use serde::{Deserialize, Serialize};

    pub use std::sync::Arc;
    pub use value::{HashMap, Value};
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
