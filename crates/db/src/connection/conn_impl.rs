use bytes::{Bytes, BytesMut};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use flow_lib::config::client::NodeDataSkipWasm;
use futures_util::StreamExt;
use utils::bs58_decode;

use super::*;

impl UserConnection {
    pub fn new(conn: Connection, wasm_storage: WasmStorage, user_id: Uuid) -> Self {
        Self {
            conn,
            user_id,
            wasm_storage,
        }
    }

    pub async fn share_flow_run(&self, id: FlowRunId, user: UserId) -> crate::Result<()> {
        // Same user, not doing anything
        if user == self.user_id {
            return Ok(());
        }

        let stmt = self
            .conn
            .prepare_cached("SELECT 1 FROM flow_run WHERE id = $1 AND user_id = $2")
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[&id, &self.user_id])
            .await
            .map_err(Error::exec("check conn permission"))?;

        let stmt = self
            .conn
            .prepare_cached(
                "INSERT INTO flow_run_shared (flow_run_id, user_id)
                VALUES ($1, $2)
                ON CONFLICT (flow_run_id, user_id) DO NOTHING",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .execute(&stmt, &[&id, &user])
            .await
            .map_err(Error::exec("insert flow_run_shared"))?;

        Ok(())
    }

    pub async fn get_flow_info(&self, flow_id: FlowId) -> crate::Result<FlowInfo> {
        let stmt = self
            .conn
            .prepare_cached(
                r#"SELECT user_id, start_shared, start_unverified FROM flows
                WHERE id = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#,
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_opt(&stmt, &[&flow_id, &self.user_id])
            .await
            .map_err(Error::exec("get_flow_info"))?
            .ok_or_else(|| Error::not_found("flow", flow_id))?
            .try_into()
    }

    pub async fn get_wallets(&self) -> crate::Result<Vec<Wallet>> {
        let stmt = self
            .conn
            .prepare_cached("SELECT public_key, keypair, id FROM wallets WHERE user_id = $1")
            .await
            .map_err(Error::exec("prepare get_wallets"))?;
        self.conn
            .query(&stmt, &[&self.user_id])
            .await
            .map_err(Error::exec("get wallets"))?
            .into_iter()
            .map(|r| {
                let pubkey_str = r
                    .try_get::<_, String>(0)
                    .map_err(Error::data("wallets.public_key"))?;
                let pubkey =
                    bs58_decode(&pubkey_str).map_err(Error::parsing("wallets.public_key"))?;

                let keypair_str = r
                    .try_get::<_, Option<String>>(1)
                    .map_err(Error::data("wallets.keypair"))?;
                let keypair = keypair_str
                    .map(|s| utils::bs58_decode(&s))
                    .transpose()
                    .map_err(Error::parsing("wallets.keypair"))?;

                let id = r.try_get(2).map_err(Error::data("wallets.id"))?;

                Ok(Wallet {
                    id,
                    pubkey,
                    keypair,
                })
            })
            .collect()
    }

    pub async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("start"))?;

        let flow_owner = {
            let stmt = tx
                .prepare_cached(
                    r#"SELECT user_id FROM flows
                    WHERE id = $1 AND (user_id = $2 OR "isPublic")"#,
                )
                .await
                .map_err(Error::exec("prepare"))?;
            let owner: UserId = tx
                .query_one(&stmt, &[&flow_id, &self.user_id])
                .await
                .map_err(Error::exec("get flow's owner"))?
                .try_get(0)
                .map_err(Error::data("flows.user_id"))?;
            owner
        };

        let get_wallets = tx
            .prepare_cached("SELECT id, public_key FROM wallets WHERE user_id = $1")
            .await
            .map_err(Error::exec("prepare"))?;
        let owner_wallets = tx
            .query(&get_wallets, &[&flow_owner])
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
            tx.query(&get_wallets, &[&self.user_id])
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
                let new_id = is_same_user
                    .then_some(wallet)
                    .or_else(|| user_wallet.iter().find(|(_, pk)| pk == owner_pk))
                    .unwrap_or_else(|| &user_wallet[0]);
                res.insert(id, new_id);
            }
            res
        };

        let mut ids = HashSet::<FlowId>::new();
        let mut queue = vec![flow_id];
        let get_interflows = tx
            .prepare_cached(
                r#"WITH nodes AS
                (
                    SELECT unnest(nodes) AS node
                    FROM flows WHERE id = $1
                )
                SELECT CAST(node #>> '{data,targets_form,form_data,id}' AS INT) AS id
                FROM nodes WHERE
                    node #>> '{data,node_id}' IN ('interflow', 'interflow_instructions')
                    AND node->>'type' = 'native'"#,
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let check_flow = tx
            .prepare_cached(
                r#"SELECT id FROM flows WHERE id = $1 AND (user_id = $2 OR "isPublic")"#,
            )
            .await
            .map_err(Error::exec("prepare"))?;
        while let Some(id) = queue.pop() {
            if tx
                .query_opt(&check_flow, &[&id, &self.user_id])
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
                .query(&get_interflows, &[&id])
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

        let stmt = tx
            .prepare(
                r#"INSERT INTO flows (
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
                    CONCAT('[CLONED] ', name) AS name,
                    mosaic,
                    CONCAT('[CLONED] ', description) AS description,
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
                RETURNING id"#,
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let mut flow_id_map = HashMap::new();
        let mut new_ids = Vec::new();
        for id in &ids {
            let new_id: i32 = tx
                .query_one(&stmt, &[id, &self.user_id])
                .await
                .map_err(Error::exec("copy flow"))?
                .try_get(0)
                .map_err(Error::data("flows.id"))?;
            flow_id_map.insert(*id, new_id);
            new_ids.push(new_id);
        }
        let stmt = tx
            .prepare(
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
                                            $3::JSONB->(node #>> '{data,targets_form,form_data,wallet_id}')->1
                                        ),
                                        '{data,targets_form,form_data,wallet_id}',
                                        $3::JSONB->(node #>> '{data,targets_form,form_data,wallet_id}')->0
                                    )

                                ELSE node
                            END

                            ORDER BY idx
                        ) AS nodes
                    FROM flows f CROSS JOIN unnest(f.nodes) WITH ORDINALITY AS n(node, idx)
                    WHERE f.id = ANY($1::INT[])
                    GROUP BY f.id
                ) AS q
                WHERE flows.id = q.id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        tx.execute(&stmt, &[&new_ids, &Json(&flow_id_map), &Json(&wallet_map)])
            .await
            .map_err(Error::exec("update interflow IDs"))?;
        tx.commit()
            .await
            .map_err(Error::exec("commit clone_flow"))?;

        Ok(flow_id_map)
    }

    pub async fn new_flow_run(
        &self,
        config: &ClientConfig,
        inputs: &ValueSet,
    ) -> crate::Result<FlowRunId> {
        let stmt = self
            .conn
            .prepare_cached(
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
                    signers)
                VALUES (
                    gen_random_uuid(),
                    $1, $2,
                    jsonb_build_object('M', $3::JSONB),
                    $4, $5,
                    jsonb_build_object('SOL', $6::JSONB),
                    $7, $8, $9, $10, $11, $12, $13)
                RETURNING id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let r = self
            .conn
            .query_one(
                &stmt,
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
                    &Json(&config.signers),
                ],
            )
            .await
            .map_err(Error::exec("new flow run"))?;
        Ok(r.get(0))
    }

    pub async fn get_previous_values(
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
        self.conn
            .query(&stmt, &[&self.user_id])
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

    pub async fn get_flow_config(&self, id: FlowId) -> crate::Result<client::ClientConfig> {
        let stmt = self
            .conn
            .prepare_cached(
                "SELECT nodes,
                        edges,
                        environment,
                        (current_network->>'url')::TEXT AS network_url,
                        (current_network->>'cluster')::TEXT AS network_cluster,
                        instructions_bundling
                FROM flows
                WHERE id = $1 AND user_id = $2",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let row = self
            .conn
            .query_opt(&stmt, &[&id, &self.user_id])
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
                if let Err(error) = self.fetch_wasm_bytes(&mut node.data.targets_form).await {
                    tracing::warn!("{}", error);
                }
            }
        }

        Ok(config)
    }

    async fn fetch_wasm_bytes(&self, data: &mut client::TargetsForm) -> crate::Result<()> {
        if data.wasm_bytes.is_some() {
            return Ok(());
        }

        let id = data
            .extra
            .supabase_id
            .ok_or_else(|| Error::not_found("json", "supabase_id"))?;

        let stmt = self
            .conn
            .prepare_cached(
                r#"SELECT storage_path FROM nodes
                WHERE id = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#,
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let path: String = self
            .conn
            .query_opt(&stmt, &[&id, &self.user_id])
            .await
            .map_err(Error::exec("get storage_path"))?
            .ok_or_else(|| Error::not_found("node", id))?
            .try_get(0)
            .map_err(Error::data("nodes.storage_path"))?;

        let bytes = self.wasm_storage.download(&path).await?;

        data.wasm_bytes = Some(bytes);

        Ok(())
    }

    pub async fn set_start_time(&self, id: &FlowRunId, time: &DateTime<Utc>) -> crate::Result<()> {
        let time = time.naive_utc();
        let stmt = self
            .conn
            .prepare_cached("UPDATE flow_run SET start_time = $1 WHERE id = $2 RETURNING id")
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[&time, id])
            .await
            .map_err(Error::exec("set start time"))?;
        Ok(())
    }

    pub async fn push_flow_error(&self, id: &FlowRunId, error: &str) -> crate::Result<()> {
        let stmt = self
            .conn
            .prepare_cached(
                "UPDATE flow_run
                SET errors = array_append(errors, $2)
                WHERE id = $1
                RETURNING id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[id, &error])
            .await
            .map_err(Error::exec("push flow errors"))?;
        Ok(())
    }

    pub async fn push_flow_log(
        &self,
        id: &FlowRunId,
        index: &i32,
        time: &DateTime<Utc>,
        level: &str,
        module: &Option<String>,
        content: &str,
    ) -> crate::Result<()> {
        let time = time.naive_utc();
        let stmt = self.conn
        .prepare_cached(
            "INSERT INTO flow_run_logs (flow_run_id, log_index, user_id, time, log_level, content, module)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING flow_run_id",
        )
        .await
        .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(
                &stmt,
                &[id, index, &self.user_id, &time, &level, &content, module],
            )
            .await
            .map_err(Error::exec("push flow log"))?;
        Ok(())
    }

    pub async fn set_run_result(
        &self,
        id: &FlowRunId,
        time: &DateTime<Utc>,
        not_run: &[NodeId],
        output: &Value,
    ) -> crate::Result<()> {
        let time = time.naive_utc();
        let stmt = self
            .conn
            .prepare_cached(
                "UPDATE flow_run
                SET end_time = $2,
                    not_run = $3,
                    output = $4
                WHERE id = $1 AND end_time IS NULL
                RETURNING id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[id, &time, &not_run, &Json(output)])
            .await
            .map_err(Error::exec("set run result"))?;
        Ok(())
    }

    pub async fn new_node_run(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
        input: &Value,
    ) -> crate::Result<()> {
        let time = time.naive_utc();
        let stmt = self
            .conn
            .prepare_cached(
                "INSERT INTO node_run (flow_run_id, node_id, times, user_id, start_time, input)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING flow_run_id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(
                &stmt,
                &[id, node_id, times, &self.user_id, &time, &Json(input)],
            )
            .await
            .map_err(Error::exec("new node run"))?;
        Ok(())
    }

    pub async fn save_node_output(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        output: &Value,
    ) -> crate::Result<()> {
        const MAP: &str = value::keys::MAP;
        let stmt = self
            .conn
            .prepare_cached(&format!(
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
                RETURNING flow_run_id"#,
            ))
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[id, node_id, times, &Json(output)])
            .await
            .map_err(Error::exec("set node finish"))?;
        Ok(())
    }

    pub async fn push_node_error(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        error: &str,
    ) -> crate::Result<()> {
        let stmt = self
            .conn
            .prepare_cached(
                "UPDATE node_run
                SET errors = array_append(errors, $4)
                WHERE flow_run_id = $1 AND node_id = $2 AND times = $3
                RETURNING flow_run_id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[id, node_id, times, &error])
            .await
            .map_err(Error::exec("push node error"))?;
        Ok(())
    }

    pub async fn push_node_log(
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
        let time = time.naive_utc();
        let stmt = self
            .conn
            .prepare_cached(
                "INSERT INTO flow_run_logs (
                    flow_run_id,
                    log_index,
                    user_id,
                    node_id,
                    times,
                    time,
                    log_level,
                    content,
                    module)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING flow_run_id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(
                &stmt,
                &[
                    id,
                    index,
                    &self.user_id,
                    node_id,
                    times,
                    &time,
                    &level,
                    &content,
                    &module,
                ],
            )
            .await
            .map_err(Error::exec("push node log"))?;
        Ok(())
    }

    pub async fn set_node_finish(
        &self,
        id: &FlowRunId,
        node_id: &NodeId,
        times: &i32,
        time: &DateTime<Utc>,
    ) -> crate::Result<()> {
        let time = time.naive_utc();
        let stmt = self
            .conn
            .prepare_cached(
                "UPDATE node_run
                SET end_time = $4
                WHERE flow_run_id = $1 AND node_id = $2 AND times = $3
                      AND end_time IS NULL
                RETURNING flow_run_id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[id, node_id, times, &time])
            .await
            .map_err(Error::exec("set node finish"))?;
        Ok(())
    }

    pub async fn new_signature_request(
        &self,
        pubkey: &[u8; 32],
        message: &[u8],
        flow_run_id: Option<&FlowRunId>,
        signatures: Option<&[Presigner]>,
    ) -> crate::Result<i64> {
        let pubkey = bs58::encode(pubkey).into_string();
        let message = base64::encode(message);
        let signatures = signatures.map(|arr| arr.iter().map(Json).collect::<Vec<_>>());
        let stmt = self
            .conn
            .prepare_cached(
                "INSERT INTO signature_requests (
                    user_id,
                    msg,
                    pubkey,
                    flow_run_id,
                    signatures
                ) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let id = self
            .conn
            .query_one(
                &stmt,
                &[&self.user_id, &message, &pubkey, &flow_run_id, &signatures],
            )
            .await
            .map_err(Error::exec("new_signature_request"))?
            .try_get(0)
            .map_err(Error::data("id"))?;

        Ok(id)
    }

    pub async fn save_signature(
        &self,
        id: &i64,
        signature: &[u8; 64],
        new_message: Option<&Bytes>,
    ) -> crate::Result<()> {
        let new_msg_base64 = new_message.map(base64::encode);
        let signature = bs58::encode(signature).into_string();
        let stmt = self
            .conn
            .prepare_cached(
                "UPDATE signature_requests
                SET signature = $1,
                    new_msg = $4
                WHERE user_id = $2 AND id = $3 AND signature IS NULL
                RETURNING id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[&signature, &self.user_id, id, &new_msg_base64])
            .await
            .map_err(Error::exec("save_signature"))?;

        Ok(())
    }

    pub async fn read_item(&self, store: &str, key: &str) -> crate::Result<Option<Value>> {
        let stmt = self
            .conn
            .prepare_cached(
                "SELECT value FROM kvstore
                WHERE user_id = $1 AND store_name = $2 AND key = $3",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let opt = self
            .conn
            .query_opt(&stmt, &[&self.user_id, &store, &key])
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

    pub async fn export_user_data(&mut self) -> crate::Result<ExportedUserData> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("start"))?;

        let stmt = tx
            .prepare_cached("SELECT pub_key FROM users_public WHERE user_id = $1")
            .await
            .map_err(Error::exec("prepare"))?;
        let pubkey = tx
            .query_one(&stmt, &[&self.user_id])
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
        let users = clear_column(users, "encrypted_password")?;
        let users = remove_column(users, "confirmed_at")?;

        let identities = copy_out(
            &tx,
            &format!(
                "SELECT * FROM auth.identities WHERE user_id = '{}'",
                self.user_id
            ),
        )
        .await?;
        let identities = remove_column(identities, "email")?;

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
        let flows = clear_column(flows, "lastest_flow_run_id")?;

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
            Ok(data) => buffer.extend_from_slice(&data[..]),
            Err(error) => return Err(Error::exec("read copy-out stream")(error)),
        }
    }
    let text = String::from_utf8(buffer.into()).map_err(Error::parsing("UTF8"))?;
    Ok(text)
}

fn clear_column(data: String, column: &str) -> crate::Result<String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(';' as u8)
        .quote('\'' as u8)
        .from_reader(data.as_bytes());
    let headers = reader
        .headers()
        .map_err(Error::parsing("csv headers"))?
        .clone();
    let col_idx = headers
        .iter()
        .position(|col| col == column)
        .ok_or_else(|| Error::not_found("column", column))?;
    let records = reader
        .records()
        .map(|r| {
            r.map_err(Error::parsing("csv iter")).map(|r| {
                r.into_iter()
                    .enumerate()
                    .map(|(i, v)| if i == col_idx { "" } else { v })
                    .collect::<StringRecord>()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut buffer = Vec::new();
    let mut writer = csv::WriterBuilder::new()
        .delimiter(';' as u8)
        .quote('\'' as u8)
        .from_writer(&mut buffer);
    writer
        .write_record(&headers)
        .map_err(Error::parsing("build csv"))?;
    for r in records {
        writer
            .write_record(&r)
            .map_err(Error::parsing("build csv"))?;
    }
    writer.flush().map_err(Error::parsing("build csv"))?;
    drop(writer);
    Ok(String::from_utf8(buffer).map_err(Error::parsing("UTF8"))?)
}

fn remove_column(data: String, column: &str) -> crate::Result<String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(';' as u8)
        .quote('\'' as u8)
        .from_reader(data.as_bytes());
    let headers = reader
        .headers()
        .map_err(Error::parsing("csv headers"))?
        .clone();
    let col_idx = headers
        .iter()
        .position(|col| col == column)
        .ok_or_else(|| Error::not_found("column", column))?;
    let records = reader
        .records()
        .map(|r| {
            r.map_err(Error::parsing("csv iter")).map(|r| {
                r.into_iter()
                    .enumerate()
                    .filter_map(|(i, v)| (i != col_idx).then_some(v))
                    .collect::<StringRecord>()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut buffer = Vec::new();
    let mut writer = csv::WriterBuilder::new()
        .delimiter(';' as u8)
        .quote('\'' as u8)
        .from_writer(&mut buffer);
    writer
        .write_record(
            &headers
                .into_iter()
                .enumerate()
                .filter_map(|(i, v)| (i != col_idx).then_some(v))
                .collect::<StringRecord>(),
        )
        .map_err(Error::parsing("build csv"))?;
    for r in records {
        writer
            .write_record(&r)
            .map_err(Error::parsing("build csv"))?;
    }
    writer.flush().map_err(Error::parsing("build csv"))?;
    drop(writer);

    Ok(String::from_utf8(buffer).map_err(Error::parsing("UTF8"))?)
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
