use client::FlowRow;

use super::super::*;

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
    pub(crate) async fn get_flow_impl(&self, id: FlowId) -> crate::Result<FlowRow> {
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

    pub(crate) async fn share_flow_run_impl(
        &self,
        id: FlowRunId,
        user: UserId,
    ) -> crate::Result<()> {
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

    pub(crate) async fn get_flow_info_impl(&self, flow_id: FlowId) -> crate::Result<FlowInfo> {
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

    pub(crate) async fn get_flow_config_impl(
        &self,
        id: FlowId,
    ) -> crate::Result<client::ClientConfig> {
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
            if node.data.r#type == CommandType::Wasm
                && let Err(error) = self
                    .fetch_wasm_bytes(&mut node.data.targets_form, &conn)
                    .await
            {
                tracing::warn!("{}", error);
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

    pub(crate) async fn clone_flow_impl(
        &mut self,
        flow_id: FlowId,
    ) -> crate::Result<HashMap<FlowId, FlowId>> {
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
}
