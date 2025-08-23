use super::prelude::*;
use crate::db_worker::{
    FindActor,
    flow_run_worker::{FlowRunWorker, WaitFinish},
};

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/output/{run_id}")
        .wrap(config.cors())
        .route(web::get().to(get_flow_output))
}

async fn get_flow_output(
    run_id: web::Path<FlowRunId>,
    auth: AuthEither<auth_v1::AuthenticatedUser, auth_v1::FlowRunToken>,
    db: web::Data<DbPool>,
) -> Result<web::Json<value::Value>, Error> {
    let run_id = run_id.into_inner();
    if !auth.can_access_flow_run(run_id, &db).await? {
        return Err(Error::custom(StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    let db_worker = DBWorker::from_registry();
    if let Some(addr) = db_worker
        .send(FindActor::<FlowRunWorker>::new(run_id))
        .await?
    {
        addr.send(WaitFinish)
            .await?
            .map_err(|_| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, "channel closed"))?;
    }
    let conn = db.get_admin_conn().await?;
    let output = conn.get_flow_run_output(run_id).await?;
    Ok(web::Json(output))
}
