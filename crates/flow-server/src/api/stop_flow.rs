use super::prelude::*;
use crate::db_worker::{
    FindActor,
    flow_run_worker::{FlowRunWorker, StopError, StopFlow},
};

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub timeout_millies: u32,
    pub reason: Option<String>,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    web::resource("/stop/{id}")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(stop_flow))
}

async fn stop_flow(
    id: web::Path<FlowRunId>,
    params: Option<web::Json<Params>>,
    user: web::ReqData<auth::JWTPayload>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<Success>, StopError> {
    let id = id.into_inner();
    let user = user.into_inner();
    let (timeout_millies, reason) = params
        .map(|p| (p.0.timeout_millies, p.0.reason))
        .unwrap_or_default();

    db_worker
        .send(FindActor::<FlowRunWorker>::new(id))
        .await?
        .ok_or(StopError::NotFound)?
        .send(StopFlow {
            user_id: user.user_id,
            run_id: id,
            timeout_millies,
            reason,
        })
        .await??;

    Ok(web::Json(Success))
}
