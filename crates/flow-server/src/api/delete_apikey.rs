use super::prelude::*;

#[derive(Deserialize)]
pub struct Params {
    key_hash: String,
}

#[derive(Serialize)]
pub struct Output {}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/delete")
        .wrap(config.cors())
        .route(web::post().to(delete_key))
}

async fn delete_key(
    params: web::Json<Params>,
    user: Auth<auth_v1::AuthenticatedUser>,
    db: web::Data<RealDbPool>,
) -> Result<web::Json<Output>, Error> {
    let Params { key_hash } = params.into_inner();
    db.get_user_conn(*user.user_id())
        .await?
        .delete_apikey(&key_hash)
        .await?;
    Ok(web::Json(Output {}))
}
