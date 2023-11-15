use super::prelude::*;
use crate::db_worker::{
    user_worker::{StartFlowFresh, StartFlowShared},
    GetUserWorker,
};
use db::pool::DbPool;
use hashbrown::HashMap;
use value::Value;

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/start_shared/{id}")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(start_flow_shared))
}

async fn start_flow_shared(
    flow_id: web::Path<FlowId>,
    params: Option<web::Json<Params>>,
    user: web::ReqData<auth::JWTPayload>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();
    let user = user.into_inner();
    let inputs = params
        .map(|web::Json(Params { inputs })| inputs)
        .unwrap_or_default();
    let inputs = inputs.into_iter().collect::<ValueSet>();

    let flow_run_id = db_worker
        .send(GetUserWorker {
            user_id: user.user_id,
            rt: actix::Arbiter::try_current().unwrap_or_else(|| {
                tracing::warn!("starting new arbiter");
                actix::Arbiter::new().handle()
            }),
        })
        .await?
        .send(StartFlowFresh {
            user: flow_lib::User { id: user.user_id },
            flow_id,
            input: inputs,
            partial_config: None,
            environment: HashMap::new(),
        })
        .await??;

    Ok(web::Json(Output { flow_run_id }))
}
