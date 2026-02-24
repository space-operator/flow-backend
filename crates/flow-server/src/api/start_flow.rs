use super::prelude::*;
use crate::db_worker::{
    GetUserWorker,
    user_worker::{StartFlowFresh, StartFlowShared},
};
use db::connection::DbClient;
use flow_lib::config::client::PartialConfig;
use hashbrown::{HashMap, HashSet};
use value::Value;

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
    pub partial_config: Option<PartialConfig>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub output_instructions: bool,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/start/{id}")
        .wrap(config.cors())
        .route(web::post().to(start_flow))
}

async fn start_flow(
    flow_id: web::Path<FlowId>,
    params: Option<web::Json<Params>>,
    user: Auth<auth_v1::AuthenticatedUser>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();
    let user_id = *user.user_id();
    let user_pubkey = *user.pubkey();
    let (inputs, partial_config, environment, output_instructions) = params
        .map(
            |web::Json(Params {
                 inputs,
                 partial_config,
                 environment,
                 output_instructions,
             })| (inputs, partial_config, environment, output_instructions),
        )
        .unwrap_or_default();
    let inputs = inputs.into_iter().collect::<ValueSet>();

    if let Some(partial_config) = &partial_config {
        tracing::debug!("partial config: {:?}", partial_config);
    }

    let flow_owner_id = {
        let conn = db.get_conn().await?;
        let row = conn
            .do_query_opt("SELECT user_id FROM flows_v2 WHERE uuid = $1", &[&flow_id])
            .await
            .map_err(DbError::exec("get flow owner"))?
            .ok_or_else(|| Error::custom(StatusCode::NOT_FOUND, "not found"))?;
        row.try_get(0).map_err(DbError::data("flows_v2.user_id"))?
    };

    let db_worker = DBWorker::from_registry();
    let flow_run_id = if flow_owner_id == user_id {
        db_worker
            .send(GetUserWorker {
                user_id,
                base_url: Some(base_url.clone()),
            })
            .await?
            .send(StartFlowFresh {
                user: flow_lib::User { id: user_id },
                flow_id,
                input: inputs,
                partial_config,
                environment,
                output_instructions,
                action_identity: None,
                action_config: None,
                fees: Vec::new(),
            })
            .await??
    } else {
        let has_adapter_access = {
            let owner_wallets = db.get_user_conn(flow_owner_id).await?.get_wallets().await?;
            let owner_adapter_pubkeys = owner_wallets
                .into_iter()
                .filter_map(|wallet| (wallet.keypair.is_none()).then_some(wallet.pubkey))
                .collect::<HashSet<_>>();

            if owner_adapter_pubkeys.contains(&user_pubkey) {
                true
            } else {
                let starter_wallets = db.get_user_conn(user_id).await?.get_wallets().await?;
                starter_wallets
                    .into_iter()
                    .filter(|wallet| wallet.keypair.is_none())
                    .any(|wallet| owner_adapter_pubkeys.contains(&wallet.pubkey))
            }
        };
        if !has_adapter_access {
            return Err(Error::custom(StatusCode::NOT_FOUND, "not found"));
        }

        let starter = db_worker
            .send(GetUserWorker {
                user_id,
                base_url: Some(base_url.clone()),
            })
            .await?;
        let owner = db_worker
            .send(GetUserWorker {
                user_id: flow_owner_id,
                base_url: Some(base_url),
            })
            .await?;

        owner
            .send(StartFlowShared {
                flow_id,
                input: inputs,
                partial_config,
                environment,
                output_instructions,
                action_identity: None,
                action_config: None,
                fees: Vec::new(),
                started_by: (user_id, starter),
            })
            .await??
    };

    Ok(web::Json(Output { flow_run_id }))
}
