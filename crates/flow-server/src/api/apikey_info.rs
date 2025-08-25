use super::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub user_id: UserId,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/info")
        .wrap(config.cors())
        .route(web::get().to(key_info))
}

async fn key_info(apikey: Auth<auth_v1::ApiKey>) -> Result<web::Json<Output>, Error> {
    let user_id = *apikey.user_id();
    Ok(web::Json(Output { user_id }))
}
