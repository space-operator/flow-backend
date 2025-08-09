use super::prelude::*;
use crate::user::{SupabaseAuth, UpsertWalletBody};
use serde_json::value::RawValue;

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/upsert")
        .wrap(config.cors())
        .route(web::post().to(upsert_wallet))
}

async fn upsert_wallet(
    params: web::Json<UpsertWalletBody>,
    token: Auth<auth_v1::Jwt>,
    sup: web::Data<SupabaseAuth>,
) -> Result<(web::Json<Box<RawValue>>, StatusCode), Error> {
    let (status, result) = sup
        .upsert_wallet(&token.token(), params.0)
        .await
        .map_err(|error| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let status = actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap();
    Ok((web::Json(result), status))
}
