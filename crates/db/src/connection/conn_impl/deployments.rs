use anyhow::anyhow;
use bytes::Bytes;
use client::FlowRow;
use flow::flow_set::{DeploymentId, Flow, FlowDeployment};
use flow_lib::{SolanaClientConfig, solana::Pubkey};
use std::{collections::BTreeSet, str::FromStr};
use tokio_postgres::{binary_copy::BinaryCopyInWriter, types::Type};

use super::super::*;

fn x402_fee_from_row(row: Row) -> crate::Result<X402Fee> {
    Ok(X402Fee {
        id: row.try_get("id").map_err(Error::data("X402Fee.id"))?,
        network: row
            .try_get("network")
            .map_err(Error::data("X402Fee.network"))?,
        pay_to: row
            .try_get("pay_to")
            .map_err(Error::data("X402Fee.pay_to"))?,
        amount: row
            .try_get("amount")
            .map_err(Error::data("X402Fee.amount"))?,
        enabled: row
            .try_get("enabled")
            .map_err(Error::data("X402Fee.enabled"))?,
    })
}

impl UserConnection {
    pub(crate) async fn get_deployment_x402_fees_impl(
        &self,
        id: &DeploymentId,
    ) -> crate::Result<Option<Vec<X402Fee>>> {
        let conn = self.pool.get_conn().await?;
        let fees = conn
            .do_query(
                r#"select
                id,
                network,
                pay_to,
                amount,
                enabled
            from flow_deployments_x402_fees
            where deployment_id = $1 and enabled"#,
                &[id],
            )
            .await
            .map_err(Error::exec("get_deployment_x402_fees"))?
            .into_iter()
            .map(x402_fee_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(if fees.is_empty() { None } else { Some(fees) })
    }

    pub(crate) async fn get_deployment_id_from_tag_impl(
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
        .ok_or_else(|| Error::not_found("deployment", format!("{entrypoint}:{tag}")))?
        .try_get::<_, Uuid>(0)
        .map_err(Error::data("flow_deployments_tags.deployment_id"))
    }

    pub(crate) async fn get_deployment_impl(
        &self,
        id: &DeploymentId,
    ) -> crate::Result<FlowDeployment> {
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
            x402_fees: None,
        };
        Ok(d)
    }

    pub(crate) async fn get_deployment_wallets_impl(
        &self,
        id: &DeploymentId,
    ) -> crate::Result<BTreeSet<i64>> {
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
            .collect::<Result<_, _>>()
            .map_err(Error::data("flow_deployments_wallets.wallet_id"))?;
        Ok(ids)
    }

    pub(crate) async fn get_deployment_flows_impl(
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

    pub(crate) async fn insert_deployment_impl(
        &self,
        d: &FlowDeployment,
    ) -> crate::Result<DeploymentId> {
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
            BinaryCopyInWriter::new(sink, &[Type::UUID, Type::UUID, Type::UUID, Type::JSONB]);
        futures_util::pin_mut!(writer);
        for f in d.flows.values() {
            let f = &f.row;
            let flow_data = f.data().map_err(Error::json("flow_deployments_flows.data"))?;
            writer
                .as_mut()
                .write(&[&id, &f.id, &f.user_id, &Json(flow_data)])
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
                d.flows.len(),
                written
            )));
        }

        tx.commit().await.map_err(Error::exec("commit"))?;

        Ok(id)
    }
}
