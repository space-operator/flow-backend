use client::{ClientConfigV2, FlowRow, FlowRowV2};
use serde_json::json;

use super::super::*;

#[track_caller]
fn row_to_flow_row(r: tokio_postgres::Row) -> crate::Result<FlowRow> {
    let Json(flow_v2) = r
        .try_get::<_, Json<FlowRowV2>>("flow")
        .map_err(Error::data("flows_v2.flow"))?;
    Ok(flow_v2.into())
}

fn is_interflow_node(node_id: &str) -> bool {
    matches!(
        node_id,
        "interflow"
            | "interflow_instructions"
            | "@spo/interflow"
            | "@spo/interflow_instructions"
            | "@spo/std.interflow.0.1"
            | "@spo/std.interflow_instructions.0.1"
    )
}

fn is_wallet_node(node_id: &str) -> bool {
    matches!(node_id, "wallet" | "@spo/wallet" | "@spo/std.wallet.0.1")
}

fn parse_flow_id(value: &JsonValue) -> Option<FlowId> {
    flow_lib::command::parse_value_tagged(value.clone())
        .ok()
        .and_then(|value| match value {
            value::Value::String(id) => Uuid::parse_str(&id).ok(),
            _ => None,
        })
}

fn set_flow_id(value: &mut JsonValue, flow_id: FlowId) {
    *value = json!({ "S": flow_id.to_string() });
}

fn parse_wallet_id(value: &JsonValue) -> Option<i64> {
    flow_lib::command::parse_value_tagged(value.clone())
        .ok()
        .and_then(|value| match value {
            value::Value::I64(id) => Some(id),
            value::Value::U64(id) => i64::try_from(id).ok(),
            _ => None,
        })
}

fn set_wallet_id(value: &mut JsonValue, wallet_id: i64) {
    *value = json!({ "U": wallet_id.to_string() });
}

fn set_public_key(value: &mut JsonValue, public_key: &str) {
    *value = json!({ "B3": public_key });
}

impl UserConnection {
    pub(crate) async fn get_flow_impl(&self, id: FlowId) -> crate::Result<FlowRow> {
        let conn = self.pool.get_conn().await?;
        let flow = conn
            .do_query_opt(
                r#"SELECT jsonb_build_object(
                        'id', uuid,
                        'user_id', user_id,
                        'nodes', nodes,
                        'edges', edges,
                        'environment', environment,
                        'current_network', current_network,
                        'instructions_bundling', instructions_bundling,
                        'is_public', "isPublic",
                        'start_shared', start_shared,
                        'start_unverified', start_unverified,
                        'current_branch_id', current_branch_id
                    ) AS flow
                FROM flows_v2
                WHERE uuid = $1 AND user_id = $2"#,
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
            r#"SELECT user_id, start_shared, start_unverified, "isPublic" FROM flows_v2
                WHERE uuid = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#,
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
                r#"SELECT jsonb_build_object(
                        'user_id', user_id,
                        'id', uuid,
                        'nodes', nodes,
                        'edges', edges,
                        'environment', environment,
                        'sol_network', current_network,
                        'instructions_bundling', instructions_bundling
                    ) AS config
                FROM flows_v2
                WHERE uuid = $1 AND user_id = $2"#,
                &[&id, &self.user_id],
            )
            .await
            .map_err(Error::exec("get_flow_config"))?
            .ok_or_else(|| Error::not_found("flow", id))?;
        let Json(config_v2) = row
            .try_get::<_, Json<ClientConfigV2>>("config")
            .map_err(Error::data("flows_v2.config"))?;
        let mut config: client::ClientConfig = config_v2.into();

        for node in &mut config.nodes {
            if let Some(wasm) = &mut node.data.wasm {
                if let Err(error) = self.fetch_wasm_bytes(wasm, &conn).await {
                    tracing::warn!("{}", error);
                }
            }
        }

        Ok(config)
    }

    async fn fetch_wasm_bytes(
        &self,
        wasm: &mut client::WasmNode,
        conn: &Connection,
    ) -> crate::Result<()> {
        if wasm.bytes.is_some() {
            return Ok(());
        }

        let path: String = conn
            .do_query_opt(
                r#"SELECT storage_path FROM node_definitions
                WHERE id = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#,
                &[&wasm.supabase_id, &self.user_id],
            )
            .await
            .map_err(Error::exec("get storage_path"))?
            .ok_or_else(|| Error::not_found("node", wasm.supabase_id))?
            .try_get(0)
            .map_err(Error::data("nodes.storage_path"))?;

        let bytes = self.wasm_storage.download(&path).await?;

        wasm.bytes = Some(bytes);

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
                    r#"SELECT user_id FROM flows_v2
                    WHERE uuid = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#,
                    &[&flow_id, &self.user_id],
                )
                .await
                .map_err(Error::exec("get flow's owner"))?
                .try_get(0)
                .map_err(Error::data("flows_v2.user_id"))?;
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
            for (owner_wallet_id, owner_pk) in &owner_wallets {
                let mapped = if is_same_user {
                    Some((*owner_wallet_id, owner_pk.clone()))
                } else {
                    user_wallet
                        .iter()
                        .find(|(_, pk)| pk == owner_pk)
                        .map(|(id, pk)| (*id, pk.clone()))
                };
                if let Some(mapped) = mapped {
                    res.insert(*owner_wallet_id, mapped);
                }
            }
            res
        };
        let default_wallet_id = user_wallet[0].0;
        let default_wallet_pubkey = user_wallet[0].1.clone();

        let mut ids = HashSet::<FlowId>::new();
        let mut queue = vec![flow_id];
        let check_flow = r#"SELECT nodes FROM flows_v2
                WHERE uuid = $1 AND (user_id = $2 OR "isPublic" = TRUE)"#;
        while let Some(id) = queue.pop() {
            if !ids.insert(id) {
                continue;
            }

            let row = tx
                .do_query_opt(check_flow, &[&id, &self.user_id])
                .await
                .map_err(Error::exec("check flow"))?
                .ok_or_else(|| {
                    Error::LogicError(anyhow::anyhow!("flow {:?} not found or not public", id))
                })?;

            let nodes: JsonValue = row
                .try_get("nodes")
                .map_err(Error::data("flows_v2.nodes"))?;
            let Some(nodes) = nodes.as_array() else {
                return Err(Error::LogicError(anyhow::anyhow!(
                    "flow {:?} has invalid nodes payload",
                    id
                )));
            };

            for node in nodes {
                let node_type = node
                    .get("type")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("native");
                if node_type != "native" {
                    continue;
                }

                let Some(node_id) = node.pointer("/data/node_id").and_then(JsonValue::as_str)
                else {
                    continue;
                };
                if !is_interflow_node(node_id) {
                    continue;
                }

                let Some(raw_interflow_id) = node.pointer("/data/config/flow_id") else {
                    continue;
                };
                let interflow_id = parse_flow_id(raw_interflow_id).ok_or_else(|| {
                    Error::LogicError(anyhow::anyhow!(
                        "invalid interflow flow_id in {:?}: {}",
                        id,
                        raw_interflow_id
                    ))
                })?;
                if !ids.contains(&interflow_id) {
                    queue.push(interflow_id);
                }
            }
        }
        let ids: Vec<FlowId> = ids.into_iter().collect();

        let copy_flow = r#"INSERT INTO flows_v2 (
                        user_id,
                        name,
                        description,
                        nodes,
                        edges,
                        viewport,
                        environment,
                        guide,
                        instructions_bundling,
                        backend_endpoint,
                        current_network,
                        start_shared,
                        start_unverified,
                        current_branch_id,
                        parent_flow,
                        linked_flows,
                        lifecycle,
                        meta_nodes,
                        default_viewport
                    ) SELECT
                        $2 AS user_id,
                        name,
                        description,
                        nodes,
                        edges,
                        viewport,
                        environment,
                        guide,
                        instructions_bundling,
                        backend_endpoint,
                        current_network,
                        start_shared,
                        start_unverified,
                        current_branch_id,
                        uuid as parent_flow,
                        linked_flows,
                        lifecycle,
                        meta_nodes,
                        default_viewport
                        FROM flows_v2 WHERE uuid = $1
                    RETURNING uuid, nodes"#;
        let mut flow_id_map = HashMap::new();
        let mut copied_nodes = HashMap::new();
        for id in &ids {
            let row = tx
                .do_query_one(copy_flow, &[id, &self.user_id])
                .await
                .map_err(Error::exec("copy flow"))?;
            let new_id: FlowId = row.try_get("uuid").map_err(Error::data("flows_v2.uuid"))?;
            let nodes: JsonValue = row
                .try_get("nodes")
                .map_err(Error::data("flows_v2.nodes"))?;
            flow_id_map.insert(*id, new_id);
            copied_nodes.insert(new_id, nodes);
        }

        for (new_id, mut nodes) in copied_nodes {
            if let Some(node_list) = nodes.as_array_mut() {
                for node in node_list {
                    let node_type = node
                        .get("type")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("native");
                    if node_type != "native" {
                        continue;
                    }

                    let Some(node_id) = node.pointer("/data/node_id").and_then(JsonValue::as_str)
                    else {
                        continue;
                    };
                    let node_id = node_id.to_owned();

                    let Some(config) = node
                        .pointer_mut("/data/config")
                        .and_then(JsonValue::as_object_mut)
                    else {
                        continue;
                    };

                    if is_interflow_node(&node_id)
                        && let Some(flow_id_value) = config.get_mut("flow_id")
                    {
                        let old_interflow_id = parse_flow_id(flow_id_value).ok_or_else(|| {
                            Error::LogicError(anyhow::anyhow!(
                                "invalid interflow flow_id in cloned flow {:?}",
                                new_id
                            ))
                        })?;
                        let mapped_interflow_id =
                            flow_id_map.get(&old_interflow_id).ok_or_else(|| {
                                Error::LogicError(anyhow::anyhow!(
                                    "missing cloned interflow target {:?}",
                                    old_interflow_id
                                ))
                            })?;
                        set_flow_id(flow_id_value, *mapped_interflow_id);
                    }

                    if is_wallet_node(&node_id)
                        && let Some(old_wallet_id) =
                            config.get("wallet_id").and_then(parse_wallet_id)
                    {
                        let (new_wallet_id, new_wallet_pubkey) = wallet_map
                            .get(&old_wallet_id)
                            .cloned()
                            .unwrap_or((default_wallet_id, default_wallet_pubkey.clone()));
                        if let Some(wallet_id_value) = config.get_mut("wallet_id") {
                            set_wallet_id(wallet_id_value, new_wallet_id);
                        } else {
                            config.insert(
                                "wallet_id".to_owned(),
                                json!({ "U": new_wallet_id.to_string() }),
                            );
                        }
                        if let Some(public_key_value) = config.get_mut("public_key") {
                            set_public_key(public_key_value, &new_wallet_pubkey);
                        } else {
                            config.insert(
                                "public_key".to_owned(),
                                json!({ "B3": new_wallet_pubkey }),
                            );
                        }
                    }
                }
            }

            tx.do_execute(
                "UPDATE flows_v2 SET nodes = $2 WHERE uuid = $1",
                &[&new_id, &Json(&nodes)],
            )
            .await
            .map_err(Error::exec("update cloned flow nodes"))?;
        }

        tx.commit()
            .await
            .map_err(Error::exec("commit clone_flow"))?;

        Ok(flow_id_map)
    }
}
