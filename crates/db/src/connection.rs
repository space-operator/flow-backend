use crate::{Error, Wallet, WasmStorage};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::Object as Connection;
use flow_lib::{
    config::client::{self, ClientConfig},
    CommandType, FlowId, FlowRunId, NodeId, UserId, ValueSet,
};
use hashbrown::{HashMap, HashSet};
use serde_json::Value as JsonValue;
use std::any::Any;
use tokio_postgres::types::Json;
use uuid::Uuid;
use value::Value;

mod admin;
pub use admin::AdminConn;
pub use admin::Password;

pub mod proxied_user_conn;

pub struct UserConnection {
    pub wasm_storage: WasmStorage,
    pub conn: Connection,
    pub user_id: Uuid,
}

#[async_trait]
pub trait UserConnectionTrait: Any + 'static {
    async fn get_flow_owner(&self, flow_id: FlowId) -> crate::Result<UserId>;

    async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>>;

    async fn get_wallets(&self) -> crate::Result<Vec<Wallet>>;

    async fn new_flow_run(
        &self,
        config: &ClientConfig,
        inputs: &ValueSet,
    ) -> crate::Result<FlowRunId>;

    async fn get_previous_values(
        &self,
        nodes: &HashMap<NodeId, FlowRunId>,
    ) -> crate::Result<HashMap<NodeId, Vec<Value>>>;

    async fn get_flow_config(&self, id: FlowId) -> crate::Result<client::ClientConfig>;

    async fn set_start_time(&self, id: &FlowRunId, time: &DateTime<Utc>) -> crate::Result<()>;

    async fn push_flow_error(&self, id: &FlowRunId, error: &str) -> crate::Result<()>;

    async fn push_flow_log(
        &self,
        id: &FlowRunId,
        index: &i32,
        time: &DateTime<Utc>,
        level: &str,
        module: &Option<String>,
        content: &str,
    ) -> crate::Result<()>;

    async fn set_run_result(
        &self,
        id: &FlowRunId,
        time: &DateTime<Utc>,
        not_run: &[NodeId],
        output: &Value,
    ) -> crate::Result<()>;

    async fn new_node_run(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        input: &Value,
    ) -> crate::Result<()>;

    async fn save_node_output(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        output: &Value,
    ) -> crate::Result<()>;

    async fn push_node_error(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        error: &str,
    ) -> crate::Result<()>;

    async fn push_node_log(
        &self,
        id: &FlowRunId,
        index: &i32,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        level: &str,
        module: &Option<String>,
        content: &str,
    ) -> crate::Result<()>;

    async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()>;

    async fn new_signature_request(&self, pubkey: &[u8; 32], message: &[u8]) -> crate::Result<i64>;

    async fn save_signature(&self, id: &i64, signature: &[u8; 64]) -> crate::Result<()>;

    async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>>;
}

#[async_trait]
impl UserConnectionTrait for UserConnection {
    async fn get_flow_owner(&self, flow_id: FlowId) -> crate::Result<UserId> {
        self.get_flow_owner(flow_id).await
    }

    async fn get_wallets(&self) -> crate::Result<Vec<Wallet>> {
        self.get_wallets().await
    }

    async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>> {
        self.clone_flow(flow_id).await
    }

    async fn new_flow_run(
        &self,
        config: &ClientConfig,
        inputs: &ValueSet,
    ) -> crate::Result<FlowRunId> {
        self.new_flow_run(config, inputs).await
    }

    async fn get_previous_values(
        &self,
        nodes: &HashMap<NodeId, FlowRunId>,
    ) -> crate::Result<HashMap<NodeId, Vec<Value>>> {
        self.get_previous_values(nodes).await
    }

    async fn get_flow_config(&self, id: FlowId) -> crate::Result<client::ClientConfig> {
        self.get_flow_config(id).await
    }

    async fn set_start_time(&self, id: &FlowRunId, time: &DateTime<Utc>) -> crate::Result<()> {
        self.set_start_time(id, time).await
    }

    async fn push_flow_error(&self, id: &FlowRunId, error: &str) -> crate::Result<()> {
        self.push_flow_error(id, error).await
    }

    async fn push_flow_log(
        &self,
        id: &FlowRunId,
        index: &i32,
        time: &DateTime<Utc>,
        level: &str,
        module: &Option<String>,
        content: &str,
    ) -> crate::Result<()> {
        self.push_flow_log(id, index, time, level, module, content)
            .await
    }

    async fn set_run_result(
        &self,
        id: &FlowRunId,
        time: &DateTime<Utc>,
        not_run: &[NodeId],
        output: &Value,
    ) -> crate::Result<()> {
        self.set_run_result(id, time, not_run, output).await
    }

    async fn new_node_run(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        input: &Value,
    ) -> crate::Result<()> {
        self.new_node_run(id, node_id, times, time, input).await
    }

    async fn save_node_output(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        output: &Value,
    ) -> crate::Result<()> {
        self.save_node_output(id, node_id, times, output).await
    }

    async fn push_node_error(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        error: &str,
    ) -> crate::Result<()> {
        self.push_node_error(id, node_id, times, error).await
    }

    async fn push_node_log(
        &self,
        id: &FlowRunId,
        index: &i32,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        level: &str,
        module: &Option<String>,
        content: &str,
    ) -> crate::Result<()> {
        self.push_node_log(id, index, node_id, times, time, level, module, content)
            .await
    }

    async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()> {
        self.set_node_finish(id, node_id, times, time).await
    }

    async fn new_signature_request(&self, pubkey: &[u8; 32], message: &[u8]) -> crate::Result<i64> {
        self.new_signature_request(pubkey, message).await
    }

    async fn save_signature(&self, id: &i64, signature: &[u8; 64]) -> crate::Result<()> {
        self.save_signature(id, signature).await
    }

    async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>> {
        self.read_item(store, key).await
    }
}

mod conn_impl;
