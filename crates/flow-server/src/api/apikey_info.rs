use super::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub user_id: UserId,
}

pub fn service(config: &Config) -> impl HttpServiceFactory {
    web::resource("/info")
        .wrap(config.cors())
        .route(web::get().to(key_info))
}

async fn key_info(
    db: web::Data<RealDbPool>,
    apikey: web::Header<auth::ApiKey>,
) -> Result<web::Json<Output>, Error> {
    let user_id = db
        .get_admin_conn()
        .await?
        .get_user_id_from_apikey(&apikey.into_inner().into_inner())
        .await?;
    Ok(web::Json(Output { user_id }))
}
