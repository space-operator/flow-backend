use crate::{
    config::{Encrypted, EncryptionKey},
    local_storage::CacheBucket,
    EncryptedWallet,
};
use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use client::FlowRow;
use deadpool_postgres::Transaction;
use flow::flow_set::{DeploymentId, Flow, FlowDeployment};
use flow_lib::{config::client::NodeDataSkipWasm, solana::Pubkey, SolanaClientConfig};
use futures_util::StreamExt;
use std::str::FromStr;
use tokio::task::spawn_blocking;
use tokio_postgres::{binary_copy::BinaryCopyInWriter, types::Type};
use utils::bs58_decode;

use super::*;

struct FlowRowCache;

impl CacheBucket for FlowRowCache {
    type Key = FlowId;
    type EncodedKey = kv::Integer;
    type Object = FlowRow;

    fn name() -> &'static str {
        "FlowRowCache"
    }

    fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
        obj.user_id == *user_id
    }

    fn encode_key(key: &Self::Key) -> Self::EncodedKey {
        kv::Integer::from(*key)
    }

    fn cache_time() -> Duration {
        Duration::from_secs(10)
    }
}

struct FlowInfoCache;

impl CacheBucket for FlowInfoCache {
    type Key = FlowId;
    type EncodedKey = kv::Integer;
    type Object = FlowInfo;

    fn name() -> &'static str {
        "FlowInfoCache"
    }

    fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
        obj.is_public || obj.user_id == *user_id
    }

    fn encode_key(key: &Self::Key) -> Self::EncodedKey {
        kv::Integer::from(*key)
    }

    fn cache_time() -> Duration {
        Duration::from_secs(10)
    }
}

struct EncryptedWalletCache;

impl CacheBucket for EncryptedWalletCache {
    type Key = UserId;
    type EncodedKey = kv::Raw;
    type Object = Vec<EncryptedWallet>;

    fn name() -> &'static str {
        "EncryptedWalletCache"
    }

    fn can_read(_: &Self::Object, _: &UserId) -> bool {
        true
    }

    fn encode_key(key: &Self::Key) -> Self::EncodedKey {
        key.as_bytes().into()
    }

    fn cache_time() -> Duration {
        Duration::from_secs(10)
    }
}

struct FlowConfigCache;

impl CacheBucket for FlowConfigCache {
    type Key = FlowId;
    type EncodedKey = kv::Integer;
    type Object = ClientConfig;

    fn name() -> &'static str {
        "FlowConfigCache"
    }

    fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
        obj.user_id == *user_id
    }

    fn encode_key(key: &Self::Key) -> Self::EncodedKey {
        kv::Integer::from(*key)
    }

    fn cache_time() -> Duration {
        Duration::from_secs(10)
    }
}

fn decrypt<I, C>(key: &EncryptionKey, encrypted: I) -> crate::Result<C>
where
    I: IntoIterator<Item = EncryptedWallet>,
    C: FromIterator<Wallet>,
{
    encrypted
        .into_iter()
        .map(|e| {
            Ok(Wallet {
                id: e.id,
                pubkey: e.pubkey,
                keypair: e
                    .encrypted_keypair
                    .map(|e| key.decrypt_keypair(&e))
                    .transpose()?
                    .map(|k| k.to_bytes()),
            })
        })
        .collect::<crate::Result<C>>()
}

#[async_trait]
impl UserConnectionTrait for UserConnection {
    async fn get_wallet_by_pubkey(&self, pubkey: &[u8; 32]) -> crate::Result<Wallet> {
        // TODO: caching
        let w = self.get_encrypted_wallet_by_pubkey(pubkey).await?;
        let key = self.pool.encryption_key()?;
        Ok(decrypt::<_, Vec<Wallet>>(key, [w])?.pop().unwrap())
    }

    async fn get_deployment_id_from_tag(
        &self,
        entrypoint: &FlowId,
        tag: &str,
    ) -> crate::Result<DeploymentId> {
        // TODO: caching
        self.get_deployment_id_from_tag_impl(entrypoint, tag).await
    }

    async fn get_deployment(&self, id: &DeploymentId) -> crate::Result<FlowDeployment> {
        // TODO: caching
        self.get_deployment_impl(id).await
    }

    async fn get_deployment_wallets(&self, id: &DeploymentId) -> crate::Result<Vec<i64>> {
        // TODO: caching
        self.get_deployment_wallets_impl(id).await
    }

    async fn get_deployment_flows(
        &self,
        id: &DeploymentId,
    ) -> crate::Result<HashMap<FlowId, Flow>> {
        // TODO: caching
        self.get_deployment_flows_impl(id).await
    }

    fn clone_connection(&self) -> Box<dyn UserConnectionTrait> {
        Box::new(self.clone())
    }

    async fn insert_deployment(&self, d: &FlowDeployment) -> crate::Result<DeploymentId> {
        self.insert_deployment_impl(d).await
    }

    async fn get_flow(&self, id: FlowId) -> crate::Result<FlowRow> {
        if let Some(cached) = self.local.get_cache::<FlowRowCache>(&self.user_id, &id) {
            return Ok(cached);
        }
        let result = self.get_flow(id).await;
        if let Ok(result) = &result {
            if let Err(error) = self.local.set_cache::<FlowRowCache>(&id, result.clone()) {
                tracing::error!("set_cache error: {}", error);
            }
        }
        result
    }

    async fn share_flow_run(&self, id: FlowRunId, user: UserId) -> crate::Result<()> {
        self.share_flow_run(id, user).await
    }

    async fn get_flow_info(&self, flow_id: FlowId) -> crate::Result<FlowInfo> {
        if let Some(cached) = self
            .local
            .get_cache::<FlowInfoCache>(&self.user_id, &flow_id)
        {
            return Ok(cached);
        }
        let result = self.get_flow_info(flow_id).await;
        if let Ok(result) = &result {
            if let Err(error) = self
                .local
                .set_cache::<FlowInfoCache>(&flow_id, result.clone())
            {
                tracing::error!("set_cache error: {}", error);
            }
        }
        result
    }

    async fn get_some_wallets(&self, ids: &[i64]) -> crate::Result<Vec<Wallet>> {
        // TODO: caching
        let key = self.pool.encryption_key()?.clone();
        let encrypted = self.get_some_wallets_impl(ids).await?;
        Ok(spawn_blocking(move || decrypt(&key, encrypted)).await??)
    }

    async fn get_wallets(&self) -> crate::Result<Vec<Wallet>> {
        let key = self.pool.encryption_key()?.clone();
        let encrypted = self.get_encrypted_wallets().await?;
        Ok(spawn_blocking(move || decrypt(&key, encrypted)).await??)
    }

    async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>> {
        self.clone_flow(flow_id).await
    }

    async fn new_flow_run(
        &self,
        config: &ClientConfig,
        inputs: &ValueSet,
        deployment_id: &Option<DeploymentId>,
    ) -> crate::Result<FlowRunId> {
        self.new_flow_run(config, inputs, &deployment_id).await
    }

    async fn get_previous_values(
        &self,
        nodes: &HashMap<NodeId, FlowRunId>,
    ) -> crate::Result<HashMap<NodeId, Vec<Value>>> {
        self.get_previous_values(nodes).await
    }

    async fn get_flow_config(&self, id: FlowId) -> crate::Result<client::ClientConfig> {
        if let Some(cached) = self.local.get_cache::<FlowConfigCache>(&self.user_id, &id) {
            return Ok(cached);
        }
        let result = self.get_flow_config(id).await;
        if let Ok(result) = &result {
            if let Err(error) = self.local.set_cache::<FlowConfigCache>(&id, result.clone()) {
                tracing::error!("set_cache error: {}", error);
            }
        }
        result
    }

    async fn set_start_time(&self, id: &FlowRunId, time: &DateTime<Utc>) -> crate::Result<()> {
        self.set_start_time(id, time).await
    }

    async fn push_flow_error(&self, id: &FlowRunId, error: &str) -> crate::Result<()> {
        self.push_flow_error(id, error).await
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

    async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()> {
        self.set_node_finish(id, node_id, times, time).await
    }

    async fn new_signature_request(
        &self,
        pubkey: &[u8; 32],
        message: &[u8],
        flow_run_id: Option<&FlowRunId>,
        signatures: Option<&[Presigner]>,
    ) -> crate::Result<i64> {
        self.new_signature_request(pubkey, message, flow_run_id, signatures)
            .await
    }

    async fn save_signature(
        &self,
        id: &i64,
        signature: &[u8; 64],
        new_message: Option<&Bytes>,
    ) -> crate::Result<()> {
        self.save_signature(id, signature, new_message).await
    }

    async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>> {
        self.read_item(store, key).await
    }

    async fn export_user_data(&mut self) -> crate::Result<ExportedUserData> {
        self.export_user_data().await
    }
}

#[track_caller]
fn row_to_flow_row(r: tokio_postgres::Row) -> crate::Result<FlowRow> {
    Ok(FlowRow {
        id: r.try_get("id").map_err(Error::data("flows.id"))?,
        user_id: r.try_get("user_id").map_err(Error::data("flows.user_id"))?,
        nodes: r
            .try_get::<_, Vec<Json<client::Node>>>("nodes")
            .map_err(Error::data("flows.nodes"))?
            .into_iter()
            .map(|x| x.0)
            .collect(),
        edges: r
            .try_get::<_, Vec<Json<client::Edge>>>("edges")
            .map_err(Error::data("flows.edges"))?
            .into_iter()
            .map(|x| x.0)
            .collect(),
        environment: r
            .try_get::<_, Json<std::collections::HashMap<String, String>>>("environment")
            .map_err(Error::data("flows.environment"))?
            .0,
        current_network: r
            .try_get::<_, Json<client::Network>>("current_network")
            .map_err(Error::data("flows.current_network"))?
            .0,
        instructions_bundling: r
            .try_get::<_, Json<client::BundlingMode>>("instructions_bundling")
            .map_err(Error::data("flows.instructions_bundling"))?
            .0,
        is_public: r
            .try_get::<_, bool>("isPublic")
            .map_err(Error::data("flows.isPublic"))?,
        start_shared: r
            .try_get::<_, bool>("start_shared")
            .map_err(Error::data("flows.start_shared"))?,
        start_unverified: r
            .try_get::<_, bool>("start_unverified")
            .map_err(Error::data("flows.start_unverified"))?,
    })
}

impl UserConnection {
    pub fn new(
        pool: RealDbPool,
        wasm_storage: WasmStorage,
        user_id: Uuid,
        local: LocalStorage,
    ) -> Self {
        Self {
            pool,
            user_id,
            wasm_storage,
            local,
        }
    }

    async fn get_encrypted_wallet_by_pubkey(
        &self,
        pubkey: &[u8; 32],
    ) -> crate::Result<EncryptedWallet> {
        let pubkey_str = bs58::encode(pubkey).into_string();
        let conn = self.pool.get_conn().await?;
        parse_encrypted_wallet(
            conn.do_query_one(
                "select public_key, encrypted_keypair, id
            from wallets where user_id = $1 and public_key = $2",
                &[&self.user_id, &pubkey_str],
            )
            .await
            .map_err(Error::exec("select wallet"))?,
        )
    }

    async fn insert_deployment_impl(&self, d: &FlowDeployment) -> crate::Result<DeploymentId> {
        if self.user_id != d.user_id {
            return Err(Error::Unauthorized);
        }
        let mut conn = self.pool.get_conn().await?;
        let tx = conn.transaction().await.map_err(Error::exec("start"))?;

        let id = DeploymentId::now_v7();
        let fees = d
            .fees
            .iter()
            .map(|(pubkey, amount)| (pubkey.to_string(), *amount))
            .collect::<Vec<_>>();
        tx.do_execute(
            "INSERT INTO flow_deployments
            (
                id,
                user_id,
                entrypoint,
                start_permission,
                output_instructions,
                action_identity,
                fees,
                solana_network
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &id,
                &d.user_id,
                &d.entrypoint,
                &Json(d.start_permission),
                &d.output_instructions,
                &d.action_identity.as_ref().map(|p| p.to_string()),
                &Json(fees),
                &Json(&d.solana_network),
            ],
        )
        .await
        .map_err(Error::exec("insert flow_deployments"))?;

        let stmt = tx
            .prepare_cached(
                "COPY flow_deployments_wallets (
                    user_id,
                    deployment_id,
                    wallet_id
                ) FROM STDIN WITH (FORMAT binary)",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let sink = tx
            .copy_in::<_, Bytes>(&stmt)
            .await
            .map_err(Error::exec("copy in"))?;
        let writer = BinaryCopyInWriter::new(sink, &[Type::UUID, Type::UUID, Type::INT8]);
        futures_util::pin_mut!(writer);
        for wallet_id in &d.wallets_id {
            writer
                .as_mut()
                .write(&[&d.user_id, &id, &wallet_id])
                .await
                .map_err(Error::exec("copy in write"))?;
        }
        let written = writer
            .finish()
            .await
            .map_err(Error::exec("copy in finish"))?;
        if written != d.wallets_id.len() as u64 {
            return Err(Error::LogicError(anyhow!(
                "size={}; written={}",
                d.wallets_id.len(),
                written
            )));
        }

        let stmt = tx
            .prepare_cached(
                "COPY flow_deployments_flows (
                    deployment_id,
                    flow_id,
                    user_id,
                    data
                ) FROM STDIN WITH (FORMAT binary)",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let sink = tx
            .copy_in::<_, Bytes>(&stmt)
            .await
            .map_err(Error::exec("copy in"))?;
        let writer =
            BinaryCopyInWriter::new(sink, &[Type::UUID, Type::INT4, Type::UUID, Type::JSONB]);
        futures_util::pin_mut!(writer);
        for f in d.flows.values() {
            let f = &f.row;
            writer
                .as_mut()
                .write(&[&id, &f.id, &f.user_id, &Json(f.data())])
                .await
                .map_err(Error::exec("copy in write"))?;
        }
        let written = writer
            .finish()
            .await
            .map_err(Error::exec("copy in finish"))?;
        if written != d.flows.len() as u64 {
            return Err(Error::LogicError(anyhow!(
                "size={}; written={}",
                d.wallets_id.len(),
                written
            )));
        }

        tx.commit().await.map_err(Error::exec("commit"))?;

        Ok(id)
    }

    async fn get_deployment_id_from_tag_impl(
        &self,
        entrypoint: &FlowId,
        tag: &str,
    ) -> crate::Result<Uuid> {
        let conn = self.pool.get_conn().await?;
        conn.do_query_opt(
            "select deployment_id from flow_deployments_tags
                where entrypoint = $1 and tag = $2",
            &[entrypoint, &tag],
        )
        .await
        .map_err(Error::exec("get_deployment_id_from_tag"))?
        .ok_or_else(|| Error::not_found("deployment", format!("{}:{}", entrypoint, tag)))?
        .try_get::<_, Uuid>(0)
        .map_err(Error::data("flow_deployments_tags.deployment_id"))
    }

    async fn get_deployment_impl(&self, id: &DeploymentId) -> crate::Result<FlowDeployment> {
        let conn = self.pool.get_conn().await?;
        const QUERY: &str = //
            r#"select
                user_id,
                entrypoint,
                start_permission,
                output_instructions,
                action_identity,
                fees,
                solana_network
            from flow_deployments
            where id = $1 and (
                (start_permission = '"Anonymous"')
            or  (start_permission = '"Authenticated"' and $2::uuid <> '00000000-0000-0000-0000-000000000000')
            or  (start_permission = '"Owner"' and $2::uuid = user_id)
            )"#;
        let r = conn
            .do_query_opt(QUERY, &[id, &self.user_id])
            .await
            .map_err(Error::exec("select flow_deployments"))?
            .ok_or_else(|| Error::not_found("flow_deployments", id))?;
        let d = FlowDeployment {
            id: *id,
            user_id: r
                .try_get("user_id")
                .map_err(Error::data("flow_deployments.entrypoint"))?,
            entrypoint: r
                .try_get("entrypoint")
                .map_err(Error::data("flow_deployments.entrypoint"))?,
            flows: Default::default(),
            start_permission: r
                .try_get::<_, Json<_>>("start_permission")
                .map_err(Error::data("flow_deployments.start_permission"))?
                .0,
            wallets_id: Default::default(),
            output_instructions: r
                .try_get("output_instructions")
                .map_err(Error::data("flow_deployments.output_instructions"))?,
            action_identity: r
                .try_get::<_, Option<&str>>("action_identity")
                .map_err(Error::data("flow_deployments.action_identity"))?
                .map(|s| {
                    s.parse::<Pubkey>()
                        .map_err(Error::parsing("flow_deployments.action_identity"))
                })
                .transpose()?,
            fees: r
                .try_get::<_, Json<Vec<(String, u64)>>>("fees")
                .map_err(Error::data("flow_deployments.fees"))?
                .0
                .into_iter()
                .map(|(pubkey, amount)| Pubkey::from_str(&pubkey).map(|pk| (pk, amount)))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Error::parsing("flow_deployments.fees"))?,
            solana_network: r
                .try_get::<_, Json<SolanaClientConfig>>("solana_network")
                .map_err(Error::data("flow_deployments.solana_network"))?
                .0,
        };
        Ok(d)
    }

    async fn get_deployment_wallets_impl(&self, id: &DeploymentId) -> crate::Result<Vec<i64>> {
        let conn = self.pool.get_conn().await?;
        let ids = conn
            .do_query(
                "SELECT wallet_id FROM flow_deployments_wallets
                WHERE deployment_id = $1 AND user_id = $2",
                &[id, &self.user_id],
            )
            .await
            .map_err(Error::exec("select flow_deployments_wallets"))?
            .into_iter()
            .map(|r| r.try_get(0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::data("flow_deployments_wallets.wallet_id"))?;
        Ok(ids)
    }

    async fn get_deployment_flows_impl(
        &self,
        id: &DeploymentId,
    ) -> crate::Result<HashMap<FlowId, Flow>> {
        fn parse(r: Row) -> crate::Result<(FlowId, Flow)> {
            let id = r
                .try_get("flow_id")
                .map_err(Error::data("flow_deployments_flows.flow_id"))?;
            let Json(flow) = r
                .try_get::<_, Json<FlowRow>>("data")
                .map_err(Error::data("flow_deployments_flows.data"))?;
            Ok((id, Flow { row: flow }))
        }

        let conn = self.pool.get_conn().await?;
        let flows = conn
            .do_query(
                "SELECT flow_id, data FROM flow_deployments_flows
            WHERE deployment_id = $1 AND user_id = $2",
                &[id, &self.user_id],
            )
            .await
            .map_err(Error::exec("select flow_deployments_flows"))?
            .into_iter()
            .map(parse)
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(flows)
    }

    async fn get_flow(&self, id: FlowId) -> crate::Result<FlowRow> {
        let conn = self.pool.get_conn().await?;
        let flow = conn
            .do_query_opt(
                r#"SELECT id,
                        user_id,
                        nodes,
                        edges,
                        environment,
                        current_network,
                        instructions_bundling,
                        "isPublic",
                        start_shared,
                        start_unverified
                FROM flows
                WHERE id = $1 AND user_id = $2"#,
                &[&id, &self.user_id],
            )
            .await
            .map_err(Error::exec("get_flow_config"))?
            .ok_or_else(|| Error::not_found("flow", id))
            .and_then(row_to_flow_row)?;

        Ok(flow)
    }

    async fn get_encrypted_wallets(&self) -> crate::Result<Vec<EncryptedWallet>> {
        if let Some(cached) = self
            .local
            .get_cache::<EncryptedWalletCache>(&self.user_id, &self.user_id)
        {
            return Ok(cached);
        }
        let result = self.get_encrypted_wallets_query().await;
        if let Ok(result) = &result {
            if let Err(error) = self
                .local
                .set_cache::<EncryptedWalletCache>(&self.user_id, result.clone())
            {
                tracing::error!("set_cache error: {}", error);
            }
        }
        result
    }

    async fn share_flow_run(&self, id: FlowRunId, user: UserId) -> crate::Result<()> {
        // Same user, not doing anything
        if user == self.user_id {
            return Ok(());
        }

        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "SELECT 1 FROM flow_run WHERE id = $1 AND user_id = $2",
            &[&id, &self.user_id],
        )
        .await
        .map_err(Error::exec("check conn permission"))?;

        conn.do_execute(
            "INSERT INTO flow_run_shared (flow_run_id, user_id)
                VALUES ($1, $2)
                ON CONFLICT (flow_run_id, user_id) DO NOTHING",
            &[&id, &user],
        )
        .await
        .map_err(Error::exec("insert flow_run_shared"))?;

        Ok(())
    }

    async fn get_flow_info(&self, flow_id: FlowId) -> crate::Result<FlowInfo> {
        let conn = self.pool.get_conn().await?;
        conn.do_query_opt(
            r#"SELECT user_id, start_shared, start_unverified, "isPublic" FROM flows
                WHERE id = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#,
            &[&flow_id, &self.user_id],
        )
        .await
        .map_err(Error::exec("get_flow_info"))?
        .ok_or_else(|| Error::not_found("flow", flow_id))?
        .try_into()
    }

    async fn get_some_wallets_impl(&self, ids: &[i64]) -> crate::Result<Vec<EncryptedWallet>> {
        let conn = self.pool.get_conn().await?;
        let result = conn
            .do_query(
                "select public_key, encrypted_keypair, id from wallets
                where id = any($1::bigint[]) and user_id = $2",
                &[&ids, &self.user_id],
            )
            .await
            .map_err(Error::exec("select wallets"))?
            .into_iter()
            .map(parse_encrypted_wallet)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    async fn get_encrypted_wallets_query(&self) -> crate::Result<Vec<EncryptedWallet>> {
        let conn = self.pool.get_conn().await?;
        let result = conn
            .do_query(
                "SELECT public_key, encrypted_keypair, id FROM wallets WHERE user_id = $1",
                &[&self.user_id],
            )
            .await
            .map_err(Error::exec("get wallets"))?
            .into_iter()
            .map(parse_encrypted_wallet)
            .collect::<crate::Result<Vec<EncryptedWallet>>>()?;

        Ok(result)
    }

    async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>> {
        let mut conn = self.pool.get_conn().await?;
        let tx = conn.transaction().await.map_err(Error::exec("start"))?;

        let flow_owner = {
            let owner: UserId = tx
                .do_query_one(
                    r#"SELECT user_id FROM flows
                    WHERE id = $1 AND (user_id = $2 OR "isPublic")"#,
                    &[&flow_id, &self.user_id],
                )
                .await
                .map_err(Error::exec("get flow's owner"))?
                .try_get(0)
                .map_err(Error::data("flows.user_id"))?;
            owner
        };

        let get_wallets = "SELECT id, public_key FROM wallets WHERE user_id = $1";
        let owner_wallets = tx
            .query(get_wallets, &[&flow_owner])
            .await
            .map_err(Error::exec("get_wallets"))?
            .into_iter()
            .map(|r| {
                Ok::<_, Error>((
                    r.try_get::<_, i64>(0).map_err(Error::data("wallets.id"))?,
                    r.try_get::<_, String>(1)
                        .map_err(Error::data("wallets.public_key"))?,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let is_same_user = self.user_id == flow_owner;
        let user_wallet = if is_same_user {
            owner_wallets.clone()
        } else {
            tx.do_query(get_wallets, &[&self.user_id])
                .await
                .map_err(Error::exec("get_wallets"))?
                .into_iter()
                .map(|r| {
                    Ok::<_, Error>((
                        r.try_get::<_, i64>(0).map_err(Error::data("wallets.id"))?,
                        r.try_get::<_, String>(1)
                            .map_err(Error::data("wallets.public_key"))?,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        if user_wallet.is_empty() {
            return Err(Error::LogicError(anyhow::anyhow!("user has no wallets")));
        }

        let wallet_map = {
            let mut res = HashMap::with_capacity(owner_wallets.len());
            for wallet in &owner_wallets {
                let (id, owner_pk) = wallet;
                let value = is_same_user
                    .then_some(wallet)
                    .or_else(|| user_wallet.iter().find(|(_, pk)| pk == owner_pk));
                if let Some(value) = value {
                    res.insert(id, value);
                }
            }
            res
        };
        let default_wallet_id = user_wallet[0].0;
        let default_wallet_pubkey = user_wallet[0].1.as_str();

        let mut ids = HashSet::<FlowId>::new();
        let mut queue = vec![flow_id];
        let get_interflows = r#"WITH nodes AS
                (
                    SELECT unnest(nodes) AS node
                    FROM flows WHERE id = $1
                )
                SELECT CAST(node #>> '{data,targets_form,form_data,id}' AS INT) AS id
                FROM nodes WHERE
                    node #>> '{data,node_id}' IN ('interflow', 'interflow_instructions')
                    AND node->>'type' = 'native'"#;
        let check_flow = r#"SELECT id FROM flows WHERE id = $1 AND (user_id = $2 OR "isPublic")"#;
        while let Some(id) = queue.pop() {
            if tx
                .do_query_opt(check_flow, &[&id, &self.user_id])
                .await
                .map_err(Error::exec("check flow"))?
                .is_some()
            {
                ids.insert(id);
            } else {
                return Err(Error::LogicError(anyhow::anyhow!(
                    "flow {:?} not found or not public",
                    id
                )));
            }

            let rows = tx
                .do_query(get_interflows, &[&id])
                .await
                .map_err(Error::exec("get interflows"))?;
            for row in rows {
                let id: i32 = row
                    .try_get(0)
                    .map_err(Error::data("data.targets_form.form_data.id"))?;
                if !ids.contains(&id) {
                    queue.push(id);
                }
            }
        }
        let ids: Vec<i32> = ids.into_iter().collect();

        let copy_flow = r#"INSERT INTO flows (
                        guide,
                        name,
                        mosaic,
                        description,
                        tags,
                        custom_networks,
                        current_network,
                        instructions_bundling,
                        environment,
                        nodes,
                        edges,
                        user_id,
                        parent_flow
                    ) SELECT
                        guide,
                        name,
                        mosaic,
                        description,
                        tags,
                        custom_networks,
                        current_network,
                        instructions_bundling,
                        environment,
                        nodes,
                        edges,
                        $2 AS user_id,
                        id as parent_flow
                        FROM flows WHERE id = $1
                    RETURNING id"#;
        let mut flow_id_map = HashMap::new();
        let mut new_ids = Vec::new();
        for id in &ids {
            let new_id: i32 = tx
                .do_query_one(copy_flow, &[id, &self.user_id])
                .await
                .map_err(Error::exec("copy flow"))?
                .try_get(0)
                .map_err(Error::data("flows.id"))?;
            flow_id_map.insert(*id, new_id);
            new_ids.push(new_id);
        }
        let update_flow =
                "UPDATE flows SET nodes = q.nodes FROM (
                    SELECT
                        f.id,
                        ARRAY_AGG(
                            CASE
                                WHEN
                                    node #>> '{data,node_id}' IN ('interflow', 'interflow_instructions')
                                    AND node->>'type' = 'native'
                                THEN jsonb_set(
                                        node,
                                        '{data,targets_form,form_data,id}',
                                        $2::JSONB->(node #>> '{data,targets_form,form_data,id}')
                                    )

                                WHEN
                                    node #>> '{data,node_id}' IN ('wallet')
                                    AND node->>'type' = 'native'
                                THEN jsonb_set(
                                        jsonb_set(
                                            node,
                                            '{data,targets_form,form_data,public_key}',
                                            COALESCE($3::JSONB->(node #>> '{data,targets_form,form_data,wallet_id}')->1, $5::JSONB)
                                        ),
                                        '{data,targets_form,form_data,wallet_id}',
                                        COALESCE($3::JSONB->(node #>> '{data,targets_form,form_data,wallet_id}')->0, $4::JSONB)
                                    )

                                ELSE node
                            END

                            ORDER BY idx
                        ) AS nodes
                    FROM flows f CROSS JOIN unnest(f.nodes) WITH ORDINALITY AS n(node, idx)
                    WHERE f.id = ANY($1::INT[])
                    GROUP BY f.id
                ) AS q
                WHERE flows.id = q.id";
        tx.do_execute(
            update_flow,
            &[
                &new_ids,
                &Json(&flow_id_map),
                &Json(&wallet_map),
                &Json(default_wallet_id),
                &Json(default_wallet_pubkey),
            ],
        )
        .await
        .map_err(Error::exec("update interflow IDs"))?;
        tx.commit()
            .await
            .map_err(Error::exec("commit clone_flow"))?;

        Ok(flow_id_map)
    }

    async fn new_flow_run(
        &self,
        config: &ClientConfig,
        inputs: &ValueSet,
        deployment_id: &Option<DeploymentId>,
    ) -> crate::Result<FlowRunId> {
        let conn = self.pool.get_conn().await?;
        let r = conn
            .do_query_one(
                "INSERT INTO flow_run (
                    id,
                    user_id,
                    flow_id,
                    inputs,
                    environment,
                    instructions_bundling,
                    network,
                    call_depth,
                    origin,
                    nodes,
                    edges,
                    collect_instructions,
                    partial_config,
                    deployment_id,
                    signers)
                VALUES (
                    gen_random_uuid(),
                    $1, $2,
                    jsonb_build_object('M', $3::JSONB),
                    $4, $5,
                    jsonb_build_object('SOL', $6::JSONB),
                    $7, $8, $9, $10, $11, $12, $13, $14)
                RETURNING id",
                &[
                    &self.user_id,
                    &config.id,
                    &Json(&inputs),
                    &Json(&config.environment),
                    &Json(&config.instructions_bundling),
                    &Json(&config.sol_network),
                    &(config.call_depth as i32),
                    &Json(&config.origin),
                    &config
                        .nodes
                        .iter()
                        .map(|n| {
                            Json(serde_json::json!({
                                "id": n.id,
                                "data": NodeDataSkipWasm::from(n.data.clone()),
                            }))
                        })
                        .collect::<Vec<_>>(),
                    &config.edges.iter().map(Json).collect::<Vec<_>>(),
                    &config.collect_instructions,
                    &config.partial_config.as_ref().map(Json),
                    &deployment_id,
                    &Json(&config.signers),
                ],
            )
            .await
            .map_err(Error::exec("new flow run"))?;
        Ok(r.get(0))
    }

    async fn get_previous_values(
        &self,
        nodes: &HashMap<NodeId, FlowRunId>,
    ) -> crate::Result<HashMap<NodeId, Vec<Value>>> {
        struct FormatArg<'a>(&'a HashMap<NodeId, FlowRunId>);
        impl std::fmt::Display for FormatArg<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut first = true;
                f.write_str("(")?;
                for (k, v) in self.0 {
                    if first {
                        first = false;
                    } else {
                        f.write_str(",")?
                    }
                    f.write_str("('")?;
                    k.fmt(f)?;
                    f.write_str("','")?;
                    v.fmt(f)?;
                    f.write_str("')")?;
                }
                f.write_str(")")?;
                Ok(())
            }
        }
        let stmt = format!(
            "SELECT
                node_id,
                ARRAY_AGG(output ORDER BY times ASC)
            FROM node_run
            WHERE
                (node_id, flow_run_id) IN {}
                AND user_id = $1
                AND output IS NOT NULL
            GROUP BY node_id",
            FormatArg(nodes)
        );
        let conn = self.pool.get_conn().await?;
        conn.query(&stmt, &[&self.user_id])
            .await
            .map_err(Error::exec("select node_run"))?
            .into_iter()
            .map(|row| {
                let node_id: Uuid = row.try_get(0).map_err(Error::data("flow_run.node_id"))?;
                let outputs: Vec<JsonValue> =
                    row.try_get(1).map_err(Error::data("flow_run.output"))?;
                let outputs: Vec<Value> = outputs
                    .into_iter()
                    .map(serde_json::from_value)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(Error::json("flow_run.output"))?;
                Ok((node_id, outputs))
            })
            .collect::<Result<HashMap<NodeId, Vec<Value>>, Error>>()
    }

    async fn get_flow_config(&self, id: FlowId) -> crate::Result<client::ClientConfig> {
        let conn = self.pool.get_conn().await?;
        let row = conn
            .do_query_opt(
                "SELECT nodes,
                        edges,
                        environment,
                        (current_network->>'url')::TEXT AS network_url,
                        (current_network->>'cluster')::TEXT AS network_cluster,
                        instructions_bundling
                FROM flows
                WHERE id = $1 AND user_id = $2",
                &[&id, &self.user_id],
            )
            .await
            .map_err(Error::exec("get_flow_config"))?
            .ok_or_else(|| Error::not_found("flow", id))?;

        let nodes = row
            .try_get::<_, Vec<JsonValue>>(0)
            .map_err(Error::data("flows.nodes"))?;

        let edges = row
            .try_get::<_, Vec<JsonValue>>(1)
            .map_err(Error::data("flows.edges"))?;

        let environment = row
            .try_get::<_, Json<HashMap<String, String>>>(2)
            .unwrap_or_else(|_| Json(HashMap::new()))
            .0;

        let network_url = row
            .try_get::<_, &str>(3)
            .map_err(Error::data("network_url"))?;

        let cluster = row
            .try_get::<_, &str>(4)
            .map_err(Error::data("network_cluster"))?;

        let instructions_bundling = row
            .try_get::<_, Json<client::BundlingMode>>(5)
            .map_err(Error::data("flows.instructions_bundling"))?
            .0;

        let config = serde_json::json!({
            "user_id": self.user_id,
            "id": id,
            "nodes": nodes,
            "edges": edges,
            "sol_network": {
                "url": network_url,
                "cluster": cluster,
            },
            "environment": environment,
            "instructions_bundling": instructions_bundling,
        });

        let mut config =
            serde_json::from_value::<client::ClientConfig>(config).map_err(Error::Deserialize)?;

        for node in &mut config.nodes {
            if node.data.r#type == CommandType::Wasm {
                if let Err(error) = self
                    .fetch_wasm_bytes(&mut node.data.targets_form, &conn)
                    .await
                {
                    tracing::warn!("{}", error);
                }
            }
        }

        Ok(config)
    }

    async fn fetch_wasm_bytes(
        &self,
        data: &mut client::TargetsForm,
        conn: &Connection,
    ) -> crate::Result<()> {
        if data.wasm_bytes.is_some() {
            return Ok(());
        }

        let id = data
            .extra
            .supabase_id
            .ok_or_else(|| Error::not_found("json", "supabase_id"))?;

        let path: String = conn
            .do_query_opt(
                r#"SELECT storage_path FROM nodes
                WHERE id = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#,
                &[&id, &self.user_id],
            )
            .await
            .map_err(Error::exec("get storage_path"))?
            .ok_or_else(|| Error::not_found("node", id))?
            .try_get(0)
            .map_err(Error::data("nodes.storage_path"))?;

        let bytes = self.wasm_storage.download(&path).await?;

        data.wasm_bytes = Some(bytes);

        Ok(())
    }

    async fn set_start_time(&self, id: &FlowRunId, time: &DateTime<Utc>) -> crate::Result<()> {
        let time = time.naive_utc();
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "UPDATE flow_run SET start_time = $1 WHERE id = $2 RETURNING id",
            &[&time, id],
        )
        .await
        .map_err(Error::exec("set start time"))?;
        Ok(())
    }

    async fn push_flow_error(&self, id: &FlowRunId, error: &str) -> crate::Result<()> {
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "UPDATE flow_run
                SET errors = array_append(errors, $2)
                WHERE id = $1
                RETURNING id",
            &[id, &error],
        )
        .await
        .map_err(Error::exec("push flow errors"))?;
        Ok(())
    }

    async fn set_run_result(
        &self,
        id: &FlowRunId,
        time: &DateTime<Utc>,
        not_run: &[NodeId],
        output: &Value,
    ) -> crate::Result<()> {
        let time = time.naive_utc();
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "UPDATE flow_run
                SET end_time = $2,
                    not_run = $3,
                    output = $4
                WHERE id = $1 AND end_time IS NULL
                RETURNING id",
            &[id, &time, &not_run, &Json(output)],
        )
        .await
        .map_err(Error::exec("set run result"))?;
        Ok(())
    }

    async fn new_node_run(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        input: &Value,
    ) -> crate::Result<()> {
        let time = time.naive_utc();
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "INSERT INTO node_run
                (flow_run_id, node_id, times, user_id, start_time, input)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING flow_run_id",
            &[id, node_id, times, &self.user_id, &time, &Json(input)],
        )
        .await
        .map_err(Error::exec("new node run"))?;
        Ok(())
    }

    async fn save_node_output(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        output: &Value,
    ) -> crate::Result<()> {
        const MAP: &str = value::keys::MAP;
        let stmt = format!(
            r#"UPDATE node_run
                SET output = COALESCE(
                    jsonb_set(
                        output,
                        '{{{MAP}}}',
                        (output->'{MAP}') || ($4::JSONB->'{MAP}')
                    ),
                    $4::JSONB
                )
                WHERE flow_run_id = $1 AND node_id = $2 AND times = $3
                RETURNING flow_run_id"#
        );
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(&stmt, &[id, node_id, times, &Json(output)])
            .await
            .map_err(Error::exec("set node finish"))?;
        Ok(())
    }

    async fn push_node_error(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        error: &str,
    ) -> crate::Result<()> {
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "UPDATE node_run
                SET errors = array_append(errors, $4)
                WHERE flow_run_id = $1 AND node_id = $2 AND times = $3
                RETURNING flow_run_id",
            &[id, node_id, times, &error],
        )
        .await
        .map_err(Error::exec("push node error"))?;
        Ok(())
    }

    async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()> {
        let time = time.naive_utc();
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "UPDATE node_run
                SET end_time = $4
                WHERE flow_run_id = $1 AND node_id = $2 AND times = $3
                      AND end_time IS NULL
                RETURNING flow_run_id",
            &[id, node_id, times, &time],
        )
        .await
        .map_err(Error::exec("set node finish"))?;
        Ok(())
    }

    async fn new_signature_request(
        &self,
        pubkey: &[u8; 32],
        message: &[u8],
        flow_run_id: Option<&FlowRunId>,
        signatures: Option<&[Presigner]>,
    ) -> crate::Result<i64> {
        let pubkey = bs58::encode(pubkey).into_string();
        let message = base64::encode(message);
        let signatures = signatures.map(|arr| arr.iter().map(Json).collect::<Vec<_>>());
        let conn = self.pool.get_conn().await?;
        let id = conn
            .do_query_one(
                "INSERT INTO signature_requests (
                    user_id,
                    msg,
                    pubkey,
                    flow_run_id,
                    signatures
                ) VALUES ($1, $2, $3, $4, $5) RETURNING id",
                &[&self.user_id, &message, &pubkey, &flow_run_id, &signatures],
            )
            .await
            .map_err(Error::exec("new_signature_request"))?
            .try_get(0)
            .map_err(Error::data("id"))?;

        Ok(id)
    }

    async fn save_signature(
        &self,
        id: &i64,
        signature: &[u8; 64],
        new_message: Option<&Bytes>,
    ) -> crate::Result<()> {
        let new_msg_base64 = new_message.map(base64::encode);
        let signature = bs58::encode(signature).into_string();
        let conn = self.pool.get_conn().await?;
        conn.do_query_one(
            "UPDATE signature_requests
                SET signature = $1,
                    new_msg = $4
                WHERE user_id = $2 AND id = $3 AND signature IS NULL
                RETURNING id",
            &[&signature, &self.user_id, id, &new_msg_base64],
        )
        .await
        .map_err(Error::exec("save_signature"))?;

        Ok(())
    }

    async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>> {
        let conn = self.pool.get_conn().await?;
        let opt = conn
            .do_query_opt(
                "SELECT value FROM kvstore
                WHERE user_id = $1 AND store_name = $2 AND key = $3",
                &[&self.user_id, &store, &key],
            )
            .await
            .map_err(Error::exec("read item kvstore"))?;
        match opt {
            Some(row) => Ok(Some(
                row.try_get::<_, Json<Value>>(0)
                    .map_err(Error::data("kvstore.value"))?
                    .0,
            )),
            None => Ok(None),
        }
    }

    async fn export_user_data(&mut self) -> crate::Result<ExportedUserData> {
        let mut conn = self.pool.get_conn().await?;
        let tx = conn.transaction().await.map_err(Error::exec("start"))?;

        let pubkey = tx
            .do_query_one(
                "SELECT pub_key FROM users_public WHERE user_id = $1",
                &[&self.user_id],
            )
            .await
            .map_err(Error::exec("get pub_key"))?
            .try_get::<_, String>(0)
            .map_err(Error::data("users_public.pub_key"))?;
        bs58_decode::<32>(&pubkey).map_err(Error::parsing("base58"))?;

        let users = copy_out(
            &tx,
            &format!("SELECT * FROM auth.users WHERE id = '{}'", self.user_id),
        )
        .await?;
        let users = csv_export::clear_column(users, "encrypted_password")?;
        let users = csv_export::remove_column(users, "confirmed_at")?;

        let nodes = copy_out(
            &tx,
            &format!(
                r#"SELECT * FROM nodes WHERE
                    user_id = '{}'
                    OR (user_id IS NULL AND "isPublic")"#,
                self.user_id
            ),
        )
        .await?;

        let identities = copy_out(
            &tx,
            &format!(
                "SELECT * FROM auth.identities WHERE user_id = '{}'",
                self.user_id
            ),
        )
        .await?;
        let identities = csv_export::remove_column(identities, "email")?;

        let pubkey_whitelists = copy_out(
            &tx,
            &format!(
                "SELECT * FROM pubkey_whitelists WHERE pubkey = '{}'",
                pubkey
            ),
        )
        .await?;

        let users_public = copy_out(
            &tx,
            &format!(
                "SELECT * FROM users_public WHERE user_id = '{}'",
                self.user_id
            ),
        )
        .await?;

        let wallets = copy_out(
            &tx,
            &format!("SELECT * FROM wallets WHERE user_id = '{}'", self.user_id),
        )
        .await?;

        let apikeys = copy_out(
            &tx,
            &format!("SELECT * FROM apikeys WHERE user_id = '{}'", self.user_id),
        )
        .await?;

        let flows = copy_out(
            &tx,
            &format!("SELECT * FROM flows WHERE user_id = '{}'", self.user_id),
        )
        .await?;
        let flows = csv_export::clear_column(flows, "lastest_flow_run_id")?;

        let user_quotas = copy_out(
            &tx,
            &format!(
                "SELECT * FROM user_quotas WHERE user_id = '{}'",
                self.user_id
            ),
        )
        .await?;

        let kvstore = copy_out(
            &tx,
            &format!("SELECT * FROM kvstore WHERE user_id = '{}'", self.user_id),
        )
        .await?;

        let kvstore_metadata = copy_out(
            &tx,
            &format!(
                "SELECT * FROM kvstore_metadata WHERE user_id = '{}'",
                self.user_id
            ),
        )
        .await?;

        tx.commit().await.map_err(Error::exec("commit"))?;
        Ok(ExportedUserData {
            user_id: self.user_id,
            users,
            identities,
            pubkey_whitelists,
            users_public,
            wallets,
            user_quotas,
            kvstore,
            kvstore_metadata,
            apikeys,
            flows,
            nodes,
        })
    }
}

fn parse_encrypted_wallet(r: Row) -> Result<EncryptedWallet, Error> {
    let pubkey_str = r
        .try_get::<_, String>(0)
        .map_err(Error::data("wallets.public_key"))?;
    let pubkey = bs58_decode(&pubkey_str).map_err(Error::parsing("wallets.public_key"))?;

    let encrypted_keypair = r
        .try_get::<_, Option<Json<Encrypted>>>(1)
        .map_err(Error::data("wallets.encrypted_keypair"))?
        .map(|json| json.0);

    let id = r.try_get(2).map_err(Error::data("wallets.id"))?;

    Ok(EncryptedWallet {
        id,
        pubkey,
        encrypted_keypair,
    })
}

async fn copy_out(tx: &Transaction<'_>, query: &str) -> crate::Result<String> {
    let query = format!(
        r#"COPY ({}) TO stdout WITH (FORMAT csv, DELIMITER ';', QUOTE '''', HEADER)"#,
        query
    );
    let stream = tx.copy_out(&query).await.map_err(Error::exec("copy-out"))?;
    futures_util::pin_mut!(stream);

    let mut buffer = BytesMut::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(data) => {
                // tracing::debug!("read {} bytes", data.len());
                buffer.extend_from_slice(&data[..]);
            }
            Err(error) => {
                // tracing::debug!("{}", String::from_utf8_lossy(&buffer));
                return Err(Error::exec("read copy-out stream")(error));
            }
        }
    }
    String::from_utf8(buffer.into()).map_err(Error::parsing("UTF8"))
}

#[cfg(test)]
mod tests {
    use crate::{config::DbConfig, pool::RealDbPool, LocalStorage, WasmStorage};
    use flow_lib::UserId;
    use serde::Deserialize;
    use toml::value::Table;

    #[tokio::test]
    #[ignore]
    async fn test_export() {
        let user_id = std::env::var("USER_ID").unwrap().parse::<UserId>().unwrap();
        let full_config: Table = toml::from_str(
            &std::fs::read_to_string(std::env::var("CONFIG_FILE").unwrap()).unwrap(),
        )
        .unwrap();
        let db_config = DbConfig::deserialize(full_config["db"].clone()).unwrap();
        let wasm = WasmStorage::new("http://localhost".parse().unwrap(), "", "").unwrap();
        let temp = tempfile::tempdir().unwrap();
        let local = LocalStorage::new(temp.path()).unwrap();
        let pool = RealDbPool::new(&db_config, wasm, local).await.unwrap();
        let mut conn = pool.get_user_conn(user_id).await.unwrap();
        let result = conn.export_user_data().await.unwrap();
        std::fs::write("/tmp/data.json", serde_json::to_vec(&result).unwrap()).unwrap();
    }
}
