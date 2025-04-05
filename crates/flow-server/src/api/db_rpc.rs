use super::prelude::*;
use serde_json::value::RawValue;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    web::resource("/db_rpc")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(db_rpc))
}

async fn db_rpc(
    params: web::Json<Box<RawValue>>,
    user: web::ReqData<auth::JWTPayload>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Result<Box<RawValue>, String>>, Error> {
    let user_id = user.into_inner().user_id;
    let result = db
        .get_user_conn(user_id)
        .await?
        .process_rpc(params.0.get())
        .await
        .map_err(|e| e.to_string());
    Ok(web::Json(result))
}
