use anyhow::anyhow;
use bytes::Bytes;
use tokio_postgres::{binary_copy::BinaryCopyInWriter, types::Type};

use super::super::*;

impl UserConnection {
    pub(crate) async fn copy_in_node_run_impl(
        &self,
        rows: Vec<PartialNodeRunRow>,
    ) -> crate::Result<()> {
        let conn = self.pool.get_conn().await?;
        let stmt = conn
            .prepare_cached(
                "copy node_run (
                    user_id,
                    flow_run_id,
                    node_id,
                    times,
                    start_time,
                    end_time,
                    input,
                    output,
                    errors
                ) from stdin with (format binary)",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let sink = conn
            .copy_in::<_, Bytes>(&stmt)
            .await
            .map_err(Error::exec("copy in"))?;
        let writer = BinaryCopyInWriter::new(
            sink,
            &[
                Type::UUID,
                Type::UUID,
                Type::UUID,
                Type::INT4,
                Type::TIMESTAMP,
                Type::TIMESTAMP,
                Type::JSONB,
                Type::JSONB,
                Type::TEXT_ARRAY,
            ],
        );
        futures_util::pin_mut!(writer);
        let len = rows.len();
        for row in rows {
            let PartialNodeRunRow {
                user_id,
                flow_run_id,
                node_id,
                times,
                start_time,
                end_time,
                input,
                output,
                errors,
            } = row;
            let start_time = start_time.map(|t| t.naive_utc());
            let end_time = end_time.map(|t| t.naive_utc());
            writer
                .as_mut()
                .write(&[
                    &user_id,
                    &flow_run_id,
                    &node_id,
                    &(times as i32),
                    &start_time,
                    &end_time,
                    &Json(input),
                    &Json(output),
                    &errors,
                ])
                .await
                .map_err(Error::exec("write copy in"))?;
        }
        let written = writer
            .finish()
            .await
            .map_err(Error::exec("finish copy in"))?;
        if written as usize != len {
            return Err(Error::LogicError(anyhow!(
                "size={}; written={}",
                len,
                written
            )));
        }
        Ok(())
    }

    pub(crate) async fn set_start_time_impl(
        &self,
        id: &FlowRunId,
        time: &DateTime<Utc>,
    ) -> crate::Result<()> {
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

    pub(crate) async fn push_flow_error_impl(
        &self,
        id: &FlowRunId,
        error: &str,
    ) -> crate::Result<()> {
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

    pub(crate) async fn set_run_result_impl(
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

    pub(crate) async fn new_node_run_impl(
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

    pub(crate) async fn save_node_output_impl(
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

    pub(crate) async fn push_node_error_impl(
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

    pub(crate) async fn set_node_finish_impl(
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

    pub(crate) async fn new_signature_request_impl(
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

    pub(crate) async fn save_signature_impl(
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

    pub(crate) async fn new_flow_run_impl(
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
                                "data": &n.data,
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

    pub(crate) async fn get_previous_values_impl(
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
}
