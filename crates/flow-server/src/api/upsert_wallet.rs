use super::prelude::*;
use crate::user::{SupabaseAuth, UpsertWalletBody};
use serde_json::value::RawValue;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/upsert")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(upsert_wallet))
}

async fn upsert_wallet(
    params: web::Json<UpsertWalletBody>,
    token: web::ReqData<auth::Token>,
    sup: web::Data<SupabaseAuth>,
) -> Result<(web::Json<Box<RawValue>>, StatusCode), Error> {
    let jwt = token
        .jwt
        .clone()
        .ok_or_else(|| Error::custom(StatusCode::BAD_REQUEST, "must be called with user's JWT"))?;
    let (status, result) = sup
        .upsert_wallet(&jwt, params.0)
        .await
        .map_err(|error| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let status = actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap();
    Ok((web::Json(result), status))
}
