use super::super::prelude::*;
use value::Value;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    web::resource("/delete_item")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(write_item))
}

#[derive(Deserialize)]
struct Params {
    store: String,
    key: String,
}

#[derive(Serialize)]
struct Output {
    old_value: Value,
}

async fn write_item(
    params: web::Json<Params>,
    user: web::ReqData<auth::JWTPayload>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Output>, Error> {
    let old_value = db
        .get_admin_conn()
        .await?
        .remove_item(&user.user_id, &params.store, &params.key)
        .await?;
    Ok(web::Json(Output { old_value }))
}
