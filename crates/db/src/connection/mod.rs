use crate::{Cache, CacheContainer, CacheValue, Error, Wallet, WasmStorage};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Object as Connection, Transaction};
use flow_lib::{
    config::client::{self, ClientConfig},
    context::signer::Presigner,
    CommandType, FlowId, FlowRunId, NodeId, UserId, ValueSet,
};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{
    any::Any,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_postgres::{
    types::{Json, ToSql},
    Error as PgError, Row,
};
use uuid::Uuid;
use value::Value;

pub mod csv_export;

mod admin;
pub use admin::AdminConn;
pub use admin::FlowRunInfo;
pub use admin::Password;

pub mod proxied_user_conn;

pub struct UserConnection {
    pub cache: CacheContainer,
    pub wasm_storage: WasmStorage,
    pub conn: Connection,
    pub user_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportedUserData {
    pub user_id: UserId,
    pub users: String,
    pub identities: String,
    pub pubkey_whitelists: String,
    pub users_public: String,
    pub wallets: String,
    pub apikeys: String,
    pub user_quotas: String,
    pub kvstore: String,
    pub kvstore_metadata: String,
    pub flows: String,
    pub nodes: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FlowInfo {
    pub user_id: Uuid,
    pub is_public: bool,
    pub start_shared: bool,
    pub start_unverified: bool,
}

impl TryFrom<Row> for FlowInfo {
    type Error = crate::Error;
    fn try_from(r: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            user_id: r.try_get("user_id").map_err(Error::data("flow.user_id"))?,
            is_public: r
                .try_get("isPublic")
                .map_err(Error::data("flow.isPublic"))?,
            start_shared: r
                .try_get("start_shared")
                .map_err(Error::data("flow.start_shared"))?,
            start_unverified: r
                .try_get("start_unverified")
                .map_err(Error::data("flow.start_unverified"))?,
        })
    }
}

#[async_trait]
pub trait UserConnectionTrait: Any + 'static {
    async fn share_flow_run(&self, id: FlowRunId, user: UserId) -> crate::Result<()>;

    async fn get_flow_info(&self, flow_id: FlowId) -> crate::Result<FlowInfo>;

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

    async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()>;

    async fn new_signature_request(
        &self,
        pubkey: &[u8; 32],
        message: &[u8],
        flow_run_id: Option<&FlowRunId>,
        signatures: Option<&[Presigner]>,
    ) -> crate::Result<i64>;

    async fn save_signature(
        &self,
        id: &i64,
        signature: &[u8; 64],
        new_msg: Option<&Bytes>,
    ) -> crate::Result<()>;

    async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>>;

    async fn export_user_data(&mut self) -> crate::Result<ExportedUserData>;
}

pub trait DbClient {
    #[track_caller]
    fn do_query_one(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, PgError>>;

    #[track_caller]
    fn do_query(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, PgError>>;

    #[track_caller]
    fn do_execute(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<u64, PgError>>;

    #[track_caller]
    fn do_query_opt(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, PgError>>;
}

impl DbClient for Connection {
    async fn do_query_one(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.query_one(&stmt, params).await
    }

    async fn do_query(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.query(&stmt, params).await
    }

    async fn do_execute(&self, stmt: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.execute(&stmt, params).await
    }

    async fn do_query_opt(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.query_opt(&stmt, params).await
    }
}

impl<'a> DbClient for Transaction<'a> {
    async fn do_query_one(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.query_one(&stmt, params).await
    }

    async fn do_query(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.query(&stmt, params).await
    }

    async fn do_execute(&self, stmt: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.execute(&stmt, params).await
    }

    async fn do_query_opt(
        &self,
        stmt: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, PgError> {
        let stmt = self.prepare_cached(stmt).await?;
        self.query_opt(&stmt, params).await
    }
}

mod conn_impl;
