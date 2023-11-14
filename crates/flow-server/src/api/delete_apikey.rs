use super::prelude::*;

#[derive(Deserialize)]
pub struct Params {
    key_hash: String,
}

#[derive(Serialize)]
pub struct Output {}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/delete")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(delete_key))
}

async fn delete_key(
    params: web::Json<Params>,
    user: web::ReqData<auth::JWTPayload>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Output>, Error> {
    let user_id = user.into_inner().user_id;
    let Params { key_hash } = params.into_inner();
    db.get_user_conn(user_id)
        .await?
        .delete_apikey(&key_hash)
        .await?;
    Ok(web::Json(Output {}))
}
