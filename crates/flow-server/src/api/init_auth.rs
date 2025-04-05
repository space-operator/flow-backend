use super::prelude::*;
use crate::user::SignatureAuth;

#[derive(Deserialize)]
pub struct Params {
    #[serde(with = "utils::serde_bs58")]
    pub pubkey: [u8; 32],
}

#[derive(Serialize)]
pub struct Output {
    pub msg: String,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/init")
        .wrap(config.anon_key())
        .wrap(config.cors())
        .route(web::post().to(init_auth))
}

async fn init_auth(
    params: web::Json<Params>,
    sig: web::Data<SignatureAuth>,
) -> Result<web::Json<Output>, Error> {
    let Params { pubkey } = params.into_inner();
    let msg = sig.init_login(chrono::Utc::now().timestamp(), &pubkey);
    Ok(web::Json(Output { msg }))
}
