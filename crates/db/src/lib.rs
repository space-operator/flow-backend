use chrono::{DateTime, Utc};
use flow_lib::{FlowRunId, NodeId, UserId};
use serde::{Deserialize, Serialize};

pub mod apikey;
pub mod config;
pub mod connection;
pub mod error;
pub mod local_storage;
pub mod pool;
pub mod wasm_storage;

pub use deadpool_postgres::Client as DeadPoolClient;
pub use error::{Error, Result};
pub use local_storage::LocalStorage;
pub use tokio_postgres::error::SqlState;
pub use wasm_storage::{StorageError, WasmStorage};

#[derive(Serialize, Deserialize)]
pub struct NodeRunRow {
    pub user_id: UserId,
    pub flow_run_id: FlowRunId,
    pub node_id: NodeId,
    pub times: i32,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub start_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub end_time: DateTime<Utc>,
    pub input: Option<value::Value>,
    pub output: Option<value::Value>,
    pub errors: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct FlowRunLogsRow {
    pub user_id: UserId,
    pub flow_run_id: FlowRunId,
    pub log_index: i32,
    pub node_id: Option<NodeId>,
    pub times: Option<i32>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub time: DateTime<Utc>,
    pub log_level: String,
    pub content: String,
    pub module: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    pub id: i64,
    #[serde(with = "utils::serde_bs58")]
    pub pubkey: [u8; 32],
    #[serde(
        default,
        with = "utils::serde_bs58::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub keypair: Option<[u8; 64]>,
}
