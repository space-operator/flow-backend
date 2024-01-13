use super::prelude::*;
use crate::{auth::ApiAuth, middleware::auth::TokenType};

#[derive(Serialize, Deserialize)]
pub struct Params {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub payload: TokenType,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    let auth = web::Data::new(config.all_auth(db));
    web::resource("/ws_auth")
        .app_data(auth)
        .wrap(config.cors())
        .route(web::post().to(ws_auth))
}

async fn ws_auth(
    params: web::Json<Params>,
    auth: web::Data<ApiAuth>,
) -> Result<web::Json<Output>, Error> {
    let payload = (*auth)
        .clone()
        .ws_authenticate(params.into_inner().token)
        .await
        .map_err(|error| Error::custom(StatusCode::UNAUTHORIZED, error))?;
    Ok(web::Json(Output { payload }))
}
