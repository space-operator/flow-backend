use super::prelude::*;
use crate::db_worker::{
    FindActor,
    flow_run_worker::{FlowRunWorker, WaitFinish},
};

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    web::resource("/output/{run_id}")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::get().to(get_flow_output))
}

async fn get_flow_output(
    run_id: web::Path<FlowRunId>,
    auth: web::ReqData<auth::TokenType>,
    db: web::Data<RealDbPool>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<value::Value>, Error> {
    let run_id = run_id.into_inner();
    let conn = db.get_admin_conn().await?;
    let run_info = conn.get_flow_run_info(run_id).await?;
    let auth = auth.into_inner();
    if !(auth.flow_run_id() == Some(run_id)
        || auth.user_id().is_some_and(|user_id| {
            run_info.user_id == user_id || run_info.shared_with.contains(&user_id)
        }))
    {
        return Err(Error::custom(StatusCode::NOT_FOUND, "unauthorized"));
    }
    if let Some(addr) = db_worker
        .send(FindActor::<FlowRunWorker>::new(run_id))
        .await?
    {
        addr.send(WaitFinish)
            .await?
            .map_err(|_| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, "channel closed"))?;
    }
    let output = conn.get_flow_run_output(run_id).await?;
    Ok(web::Json(output))
}
