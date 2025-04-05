use super::super::prelude::*;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    web::resource("/delete_store")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(delete_store))
}

#[derive(Deserialize)]
struct Params {
    store: String,
}

async fn delete_store(
    params: web::Json<Params>,
    user: web::ReqData<auth::JWTPayload>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Success>, Error> {
    let params = params.into_inner();
    let success = db
        .get_admin_conn()
        .await?
        .delete_store(&user.user_id, &params.store)
        .await?;
    if success {
        Ok(web::Json(Success))
    } else {
        Err(Error::custom(StatusCode::NOT_FOUND, "store not found"))
    }
}
