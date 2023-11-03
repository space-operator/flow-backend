use super::prelude::*;
use crate::db_worker::CopyIn;
use db::FlowRunLogsRow;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/db_push_logs")
        .wrap(config.apikey_auth(db))
        .wrap(config.cors())
        .route(web::post().to(db_push_logs))
}

async fn db_push_logs(
    params: web::Json<Vec<FlowRunLogsRow>>,
    user: web::ReqData<auth::JWTPayload>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<Success>, Error> {
    let user_id = user.into_inner().user_id;
    let rows = params
        .into_inner()
        .into_iter()
        .filter(|row| row.user_id == user_id)
        .collect::<Vec<_>>();
    db_worker.send(CopyIn(rows)).await?;
    Ok(web::Json(Success))
}
