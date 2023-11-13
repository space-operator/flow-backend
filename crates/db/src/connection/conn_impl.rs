use flow_lib::config::client::NodeDataSkipWasm;
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

    pub async fn get_wallets(&self) -> crate::Result<Vec<Wallet>> {
        let stmt = self
            .conn
            .prepare_cached("SELECT public_key, keypair FROM wallets WHERE user_id = $1")
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

                Ok(Wallet { pubkey, keypair })
            })
            .collect()
    }

    pub async fn clone_flow(&mut self, flow_id: FlowId) -> crate::Result<HashMap<FlowId, FlowId>> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("start"))?;
        let mut ids = HashSet::<FlowId>::new();
        let mut queue = vec![flow_id];
        let stmt = tx
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
        let stmt1 = tx
            .prepare_cached(
                r#"SELECT id FROM flows WHERE id = $1 AND (user_id = $2 OR "isPublic")"#,
            )
            .await
            .map_err(Error::exec("prepare"))?;
        while let Some(id) = queue.pop() {
            if tx
                .query_opt(&stmt1, &[&id, &self.user_id])
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
                .query(&stmt, &[&id])
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
                    user_id
                ) SELECT
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
                    $2 AS user_id
                    FROM flows WHERE id = $1
                RETURNING id"#,
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let mut map = HashMap::new();
        let mut new_ids = Vec::new();
        for id in &ids {
            let new_id: i32 = tx
                .query_one(&stmt, &[id, &self.user_id])
                .await
                .map_err(Error::exec("copy flow"))?
                .try_get(0)
                .map_err(Error::data("flows.id"))?;
            map.insert(*id, new_id);
            new_ids.push(new_id);
        }
        let stmt = tx
            .prepare(
                "UPDATE flows SET nodes = q.nodes FROM (
                    SELECT
                        f.id,
                        ARRAY_AGG(
                            CASE WHEN
                                node #>> '{data,node_id}' IN ('interflow', 'interflow_instructions')
                                AND node->>'type' = 'native'
                            THEN jsonb_set(
                                    node,
                                    '{data,targets_form,form_data,id}',
                                    $2::JSONB->(node #>> '{data,targets_form,form_data,id}')
                                )
                            ELSE node END
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
        tx.execute(&stmt, &[&new_ids, &Json(&map)])
            .await
            .map_err(Error::exec("update interflow IDs"))?;
        tx.commit()
            .await
            .map_err(Error::exec("commit clone_flow"))?;

        Ok(map)
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
                    partial_config)
                VALUES (
                    gen_random_uuid(),
                    $1, $2,
                    jsonb_build_object('M', $3::JSONB),
                    $4, $5,
                    jsonb_build_object('SOL', $6::JSONB),
                    $7, $8, $9, $10, $11, $12)
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
    ) -> crate::Result<i64> {
        let pubkey = bs58::encode(pubkey).into_string();
        let message = base64::encode(message);
        let stmt = self
            .conn
            .prepare_cached(
                "INSERT INTO signature_requests (
                    user_id,
                    msg,
                    pubkey
                ) VALUES ($1, $2, $3) RETURNING id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let id = self
            .conn
            .query_one(&stmt, &[&self.user_id, &message, &pubkey])
            .await
            .map_err(Error::exec("new_signature_request"))?
            .try_get(0)
            .map_err(Error::data("id"))?;

        Ok(id)
    }

    pub async fn save_signature(&self, id: &i64, signature: &[u8; 64]) -> crate::Result<()> {
        let signature = bs58::encode(signature).into_string();
        let stmt = self
            .conn
            .prepare_cached(
                "UPDATE signature_requests
                SET signature = $1
                WHERE user_id = $2 AND id = $3 AND signature IS NULL
                RETURNING id",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_one(&stmt, &[&signature, &self.user_id, id])
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
}
