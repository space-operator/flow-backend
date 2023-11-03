use super::prelude::*;
use crate::db_worker::{
    flow_run_worker::{FlowRunWorker, StopError, StopFlow},
    FindActor,
};

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub timeout_millies: u32,
}

#[derive(Serialize)]
pub struct Output {
    pub success: bool,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/stop/{id}")
        .wrap(config.apikey_auth(db))
        .wrap(config.cors())
        .route(web::post().to(stop_flow))
}

async fn stop_flow(
    id: web::Path<FlowRunId>,
    params: Option<web::Json<Params>>,
    user: web::ReqData<auth::JWTPayload>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<Output>, StopError> {
    let id = id.into_inner();
    let user = user.into_inner();
    let timeout_millies = params
        .map(|web::Json(Params { timeout_millies })| timeout_millies)
        .unwrap_or_default();

    db_worker
        .send(FindActor::<FlowRunWorker>::new(id))
        .await?
        .ok_or(StopError::NotFound)?
        .send(StopFlow {
            user_id: user.user_id,
            run_id: id,
            timeout_millies,
        })
        .await??;

    Ok(web::Json(Output { success: true }))
}
