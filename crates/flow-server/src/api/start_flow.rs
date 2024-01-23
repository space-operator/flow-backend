use super::prelude::*;
use crate::db_worker::{user_worker::StartFlowFresh, GetUserWorker};
use db::pool::DbPool;
use flow_lib::config::client::PartialConfig;
use hashbrown::HashMap;
use value::Value;

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
    #[serde(default)]
    pub partial_config: Option<PartialConfig>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/start/{id}")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(start_flow))
}

async fn start_flow(
    flow_id: web::Path<FlowId>,
    params: Option<web::Json<Params>>,
    user: web::ReqData<auth::JWTPayload>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();
    let user = user.into_inner();
    let (inputs, partial_config, environment) = params
        .map(
            |web::Json(Params {
                 inputs,
                 partial_config,
                 environment,
             })| (inputs, partial_config, environment),
        )
        .unwrap_or_default();
    let inputs = inputs.into_iter().collect::<ValueSet>();

    let flow_run_id = db_worker
        .send(GetUserWorker {
            user_id: user.user_id,
        })
        .await?
        .send(StartFlowFresh {
            user: flow_lib::User { id: user.user_id },
            flow_id,
            input: inputs,
            partial_config,
            environment,
        })
        .await??;

    Ok(web::Json(Output { flow_run_id }))
}
