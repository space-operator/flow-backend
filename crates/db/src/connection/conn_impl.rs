use crate::{
    EncryptedWallet,
    config::{Encrypted, EncryptionKey},
    local_storage::CacheBucket,
};
use bytes::{Bytes, BytesMut};
use client::FlowRow;
use deadpool_postgres::Transaction;
use flow::flow_set::{DeploymentId, Flow, FlowDeployment};
use futures_util::StreamExt;
use polars::{error::PolarsError, frame::DataFrame, series::Series};
use std::collections::BTreeSet;
use tokio::task::spawn_blocking;
use utils::bs58_decode;

use super::*;

mod deployments;
mod flow_run_states;
mod flows;
mod wallets;

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

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct WithUser<T> {
    pub(crate) user_id: UserId,
    pub(crate) value: T,
}

#[async_trait(?Send)]
impl UserConnectionTrait for UserConnection {
    async fn copy_in_node_run(&self, rows: Vec<PartialNodeRunRow>) -> crate::Result<()> {
        self.copy_in_node_run_impl(rows).await
    }

    async fn create_apikey(&self, name: &str) -> Result<(APIKey, String), Error<NameConflict>> {
        self.create_apikey_impl(name).await
    }

    async fn delete_apikey(&self, key_hash: &str) -> crate::Result<()> {
        self.delete_apikey_impl(key_hash).await
    }

    async fn get_wallet_by_pubkey(&self, pubkey: &[u8; 32]) -> crate::Result<Wallet> {
        struct WalletByPubkeyCache;
        impl CacheBucket for WalletByPubkeyCache {
            type Key = [u8; 32];
            type EncodedKey = kv::Raw;
            type Object = WithUser<EncryptedWallet>;

            fn name() -> &'static str {
                "WalletByPubkeyCache"
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                key.into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(60 * 24)
            }

            fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
                obj.user_id == *user_id
            }
        }

        if let Some(cached) = self
            .local
            .get_cache::<WalletByPubkeyCache>(&self.user_id, pubkey)
        {
            let key = self.pool.encryption_key()?;
            let wallet = decrypt::<_, Vec<Wallet>>(key, [cached.value])?
                .pop()
                .ok_or_else(|| Error::LogicError(anyhow::anyhow!("wallet cache decrypt empty")))?;
            return Ok(wallet);
        }

        let key = self.pool.encryption_key()?;
        let w = self.get_encrypted_wallet_by_pubkey(pubkey).await?;
        let wallet = decrypt::<_, Vec<Wallet>>(key, [w.clone()])?
            .pop()
            .ok_or_else(|| Error::LogicError(anyhow::anyhow!("wallet decrypt empty")))?;

        if let Err(error) = self.local.set_cache::<WalletByPubkeyCache>(
            pubkey,
            WithUser {
                user_id: self.user_id,
                value: w,
            },
        ) {
            tracing::error!("set_cache error: {}", error);
        }

        Ok(wallet)
    }

    async fn get_deployment_x402_fees(
        &self,
        id: &DeploymentId,
    ) -> crate::Result<Option<Vec<X402Fee>>> {
        self.get_deployment_x402_fees_impl(id).await
    }

    async fn get_deployment_id_from_tag(
        &self,
        entrypoint: &FlowId,
        tag: &str,
    ) -> crate::Result<DeploymentId> {
        self.get_deployment_id_from_tag_impl(entrypoint, tag).await
    }

    async fn get_deployment(&self, id: &DeploymentId) -> crate::Result<FlowDeployment> {
        struct FlowDeploymentCache;
        impl CacheBucket for FlowDeploymentCache {
            type Key = DeploymentId;
            type EncodedKey = kv::Raw;
            type Object = FlowDeployment;

            fn name() -> &'static str {
                "FlowDeploymentCache"
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                key.as_bytes().into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(60 * 24 * 7)
            }

            fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
                obj.user_can_read(user_id)
            }
        }

        self.run_auto_cache::<FlowDeploymentCache>(id, async |this| {
            this.get_deployment_impl(id).await
        })
        .await
    }

    async fn get_deployment_wallets(&self, id: &DeploymentId) -> crate::Result<BTreeSet<i64>> {
        struct DeploymentWalletsCache;
        impl CacheBucket for DeploymentWalletsCache {
            type Key = DeploymentId;
            type EncodedKey = kv::Raw;
            type Object = BTreeSet<i64>;

            fn name() -> &'static str {
                "DeploymentWalletsCache"
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                key.as_bytes().into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(60 * 24 * 7)
            }

            fn can_read(_: &Self::Object, _: &UserId) -> bool {
                true
            }
        }

        self.run_auto_cache::<DeploymentWalletsCache>(id, async |this| {
            this.get_deployment_wallets_impl(id).await
        })
        .await
    }

    async fn get_deployment_flows(
        &self,
        id: &DeploymentId,
    ) -> crate::Result<HashMap<FlowId, Flow>> {
        struct DeploymentFlowsCache;
        impl CacheBucket for DeploymentFlowsCache {
            type Key = DeploymentId;
            type EncodedKey = kv::Raw;
            type Object = HashMap<FlowId, Flow>;

            fn name() -> &'static str {
                "DeploymentFlowsCache"
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                key.as_bytes().into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(60 * 24 * 7)
            }

            fn can_read(_: &Self::Object, _: &UserId) -> bool {
                true
            }
        }

        self.run_auto_cache::<DeploymentFlowsCache>(id, async |this| {
            this.get_deployment_flows_impl(id).await
        })
        .await
    }

    fn clone_connection(&self) -> Box<dyn UserConnectionTrait> {
        Box::new(self.clone())
    }

    async fn insert_deployment(&self, d: &FlowDeployment) -> crate::Result<DeploymentId> {
        self.insert_deployment_impl(d).await
    }

    async fn get_flow(&self, id: FlowId) -> crate::Result<FlowRow> {
        struct FlowRowCache;
        impl CacheBucket for FlowRowCache {
            type Key = FlowId;
            type EncodedKey = kv::Raw;
            type Object = FlowRow;

            fn name() -> &'static str {
                "FlowRowCache"
            }

            fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
                obj.user_id == *user_id
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                key.as_bytes().into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(10)
            }
        }

        self.run_auto_cache::<FlowRowCache>(&id, async |this| this.get_flow_impl(id).await)
            .await
    }

    async fn share_flow_run(&self, id: FlowRunId, user: UserId) -> crate::Result<()> {
        self.share_flow_run_impl(id, user).await
    }

    async fn get_flow_info(&self, flow_id: FlowId) -> crate::Result<FlowInfo> {
        struct FlowInfoCache;
        impl CacheBucket for FlowInfoCache {
            type Key = FlowId;
            type EncodedKey = kv::Raw;
            type Object = FlowInfo;

            fn name() -> &'static str {
                "FlowInfoCache"
            }

            fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
                obj.is_public || obj.user_id == *user_id
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                key.as_bytes().into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(10)
            }
        }

        self.run_auto_cache::<FlowInfoCache>(&flow_id, async |this| {
            this.get_flow_info_impl(flow_id).await
        })
        .await
    }

    async fn get_some_wallets(&self, ids: &[i64]) -> crate::Result<Vec<Wallet>> {
        struct GetSomeWalletsCache;
        impl CacheBucket for GetSomeWalletsCache {
            type Key = i64;
            type EncodedKey = kv::Integer;
            type Object = WithUser<EncryptedWallet>;

            fn name() -> &'static str {
                "GetSomeWalletsCache"
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                (*key as u64).into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(60 * 24)
            }

            fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
                obj.user_id == *user_id
            }
        }

        let cached = ids
            .iter()
            .filter_map(|id| {
                self.local
                    .get_cache::<GetSomeWalletsCache>(&self.user_id, id)
                    .map(|w| w.value)
            })
            .collect::<Vec<_>>();

        let ids = ids
            .iter()
            .copied()
            .filter(|id| !cached.iter().any(|w| w.id == *id))
            .collect::<Vec<_>>();

        let key = self.pool.encryption_key()?.clone();
        let mut encrypted = self.get_some_wallets_impl(&ids).await?;

        for w in &encrypted {
            if let Err(error) = self.local.set_cache::<GetSomeWalletsCache>(
                &w.id,
                WithUser {
                    user_id: self.user_id,
                    value: w.clone(),
                },
            ) {
                tracing::error!("set_cache error: {}", error);
            }
        }

        encrypted.extend(cached);

        Ok(spawn_blocking(move || decrypt(&key, encrypted)).await??)
    }

    async fn get_wallets(&self) -> crate::Result<Vec<Wallet>> {
        let key = self.pool.encryption_key()?.clone();
        let encrypted = self.get_encrypted_wallets_impl().await?;
        Ok(spawn_blocking(move || decrypt(&key, encrypted)).await??)
    }

    async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>> {
        self.clone_flow_impl(flow_id).await
    }

    async fn new_flow_run(
        &self,
        config: &ClientConfig,
        inputs: &ValueSet,
        deployment_id: &Option<DeploymentId>,
    ) -> crate::Result<FlowRunId> {
        self.new_flow_run_impl(config, inputs, deployment_id).await
    }

    async fn get_previous_values(
        &self,
        nodes: &HashMap<NodeId, FlowRunId>,
    ) -> crate::Result<HashMap<NodeId, Vec<Value>>> {
        self.get_previous_values_impl(nodes).await
    }

    async fn get_flow_config(&self, id: FlowId) -> crate::Result<client::ClientConfig> {
        struct FlowConfigCache;
        impl CacheBucket for FlowConfigCache {
            type Key = FlowId;
            type EncodedKey = kv::Raw;
            type Object = ClientConfig;

            fn name() -> &'static str {
                "FlowConfigCache"
            }

            fn can_read(obj: &Self::Object, user_id: &UserId) -> bool {
                obj.user_id == *user_id
            }

            fn encode_key(key: &Self::Key) -> Self::EncodedKey {
                key.as_bytes().into()
            }

            fn cache_time() -> Duration {
                Duration::from_secs(10)
            }
        }

        self.run_auto_cache::<FlowConfigCache>(&id, async |this| {
            this.get_flow_config_impl(id).await
        })
        .await
    }

    async fn set_start_time(&self, id: &FlowRunId, time: &DateTime<Utc>) -> crate::Result<()> {
        self.set_start_time_impl(id, time).await
    }

    async fn push_flow_error(&self, id: &FlowRunId, error: &str) -> crate::Result<()> {
        self.push_flow_error_impl(id, error).await
    }

    async fn set_run_result(
        &self,
        id: &FlowRunId,
        time: &DateTime<Utc>,
        not_run: &[NodeId],
        output: &Value,
    ) -> crate::Result<()> {
        self.set_run_result_impl(id, time, not_run, output).await
    }

    async fn new_node_run(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        input: &Value,
    ) -> crate::Result<()> {
        self.new_node_run_impl(id, node_id, times, time, input)
            .await
    }

    async fn save_node_output(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        output: &Value,
    ) -> crate::Result<()> {
        self.save_node_output_impl(id, node_id, times, output).await
    }

    async fn push_node_error(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        error: &str,
    ) -> crate::Result<()> {
        self.push_node_error_impl(id, node_id, times, error).await
    }

    async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()> {
        self.set_node_finish_impl(id, node_id, times, time).await
    }

    async fn new_signature_request(
        &self,
        pubkey: &[u8; 32],
        message: &[u8],
        flow_run_id: Option<&FlowRunId>,
        signatures: Option<&[Presigner]>,
    ) -> crate::Result<i64> {
        self.new_signature_request_impl(pubkey, message, flow_run_id, signatures)
            .await
    }

    async fn save_signature(
        &self,
        id: &i64,
        signature: &[u8; 64],
        new_message: Option<&Bytes>,
    ) -> crate::Result<()> {
        self.save_signature_impl(id, signature, new_message).await
    }

    async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>> {
        self.read_item(store, key).await
    }

    async fn export_user_data(&mut self) -> crate::Result<ExportedUserData> {
        self.export_user_data().await
    }
}

impl UserConnection {
    pub fn new(
        pool: DbPool,
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

    pub(crate) async fn run_auto_cache<C>(
        &self,
        key: &C::Key,
        run: impl AsyncFnOnce(&Self) -> crate::Result<C::Object>,
    ) -> crate::Result<C::Object>
    where
        C: CacheBucket,
    {
        if let Some(cached) = self.local.get_cache::<C>(&self.user_id, key) {
            return Ok(cached);
        }
        let result = run(self).await;
        if let Ok(result) = &result
            && let Err(error) = self.local.set_cache::<C>(key, result.clone())
        {
            tracing::error!("set_cache error: {}", error);
        }
        result
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

        let mut users = copy_out(
            &tx,
            &format!("SELECT * FROM auth.users WHERE id = '{}'", self.user_id),
        )
        .await?;
        csv_export::clear_column(&mut users, "encrypted_password")?;
        users.drop_in_place("confirmed_at")?;

        let node_definitions = copy_out(
            &tx,
            &format!(
                r#"SELECT id, version, name, type, ports, config, config_schema, author_handle, user_id
                    FROM node_definitions WHERE
                    user_id = '{}'
                    OR (user_id IS NULL AND is_published = true)"#,
                self.user_id
            ),
        )
        .await?;

        let mut identities = copy_out(
            &tx,
            &format!(
                "SELECT * FROM auth.identities WHERE user_id = '{}'",
                self.user_id
            ),
        )
        .await?;
        identities.drop_in_place("email")?;

        let pubkey_whitelists = copy_out(
            &tx,
            &format!("SELECT * FROM pubkey_whitelists WHERE pubkey = '{pubkey}'"),
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
        let key = self.pool.encryption_key()?.clone();
        let wallets = spawn_blocking(move || decrypt_wallets_df(wallets, key)).await??;

        let apikeys = copy_out(
            &tx,
            &format!("SELECT * FROM apikeys WHERE user_id = '{}'", self.user_id),
        )
        .await?;

        let mut flows = copy_out(
            &tx,
            &format!("SELECT * FROM flows_v2 WHERE user_id = '{}'", self.user_id),
        )
        .await?;
        flows.drop_in_place("current_branch_id").ok();
        flows.drop_in_place("backend_endpoint").ok();
        flows.drop_in_place("slug").ok();
        flows.drop_in_place("visibility_overrides").ok();
        flows.drop_in_place("visibility_profile_id").ok();
        flows.drop_in_place("canvas_state").ok();
        flows.drop_in_place("variables").ok();
        flows.drop_in_place("rpc_id").ok();
        if flows
            .get_column_names()
            .iter()
            .any(|name| name.as_str() == "lastest_flow_run_id")
        {
            csv_export::clear_column(&mut flows, "lastest_flow_run_id")?;
        }

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
            node_definitions,
        })
    }
}

fn decrypt_wallets_df(mut wallets: DataFrame, key: EncryptionKey) -> Result<DataFrame, Error> {
    wallets.try_apply("encrypted_keypair", |series| {
        let str_series = series.str()?;
        str_series
            .iter()
            .map(|opt| {
                opt.map(|s| {
                    let encrypted: Encrypted =
                        serde_json::from_str(s).map_err(Error::json("encrypted_keypair"))?;
                    Ok(key.decrypt_keypair(&encrypted)?.to_base58_string())
                })
                .transpose()
            })
            .collect::<Result<Series, Error>>()
            .map_err(|error| PolarsError::ComputeError(error.to_string().into()))
    })?;
    wallets.rename("encrypted_keypair", "keypair".into())?;
    Ok(wallets)
}

async fn copy_out(tx: &Transaction<'_>, query: &str) -> crate::Result<DataFrame> {
    let query =
        format!(r#"COPY ({query}) TO stdout WITH (FORMAT csv, DELIMITER ';', QUOTE '''', HEADER)"#);
    let stream = tx.copy_out(&query).await.map_err(Error::exec("copy-out"))?;
    futures_util::pin_mut!(stream);

    let mut buffer = BytesMut::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(data) => {
                buffer.extend_from_slice(&data[..]);
            }
            Err(error) => {
                return Err(Error::exec("read copy-out stream")(error));
            }
        }
    }
    Ok(csv_export::read_df(&buffer)?)
}
