use super::prelude::{
    auth::{JWTPayload, Token},
    *,
};

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub payload: JWTPayload,
    pub token: Token,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    web::resource("/auth")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(auth))
}

async fn auth(
    payload: web::ReqData<JWTPayload>,
    token: web::ReqData<Token>,
) -> Result<web::Json<Output>, Error> {
    let payload = payload.into_inner();
    let token = token.into_inner();
    Ok(web::Json(Output { payload, token }))
}
