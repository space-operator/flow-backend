use super::prelude::*;
use crate::user::{SignatureAuth, SupabaseAuth};
use serde_json::value::RawValue;

#[derive(Deserialize)]
pub struct Params {
    token: String,
}

#[derive(Serialize)]
pub struct Output {
    /// Response from Supabase's login API
    session: Box<RawValue>,
    /// True if this is a new user
    new_user: bool,
}

pub fn service(config: &Config) -> impl HttpServiceFactory {
    web::resource("/confirm")
        .wrap(config.anon_key())
        .wrap(config.cors())
        .route(web::post().to(confirm_auth))
}

async fn confirm_auth(
    params: web::Json<Params>,
    sig: web::Data<SignatureAuth>,
    sup: web::Data<SupabaseAuth>,
) -> Result<web::Json<Output>, Error> {
    let Params { token } = params.into_inner();
    tracing::debug!("comfirming signature");
    let payload = sig.confirm(chrono::Utc::now().timestamp(), &token)?;
    tracing::debug!("logining to supabase");
    let (session, new_user) = sup.login(&payload).await?;
    Ok(web::Json(Output { session, new_user }))
}
