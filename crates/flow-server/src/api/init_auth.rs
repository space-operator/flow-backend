use super::bs58_decode;
use super::prelude::*;
use crate::user::SignatureAuth;

#[derive(Deserialize)]
pub struct Params {
    #[serde(deserialize_with = "bs58_decode")]
    pub pubkey: [u8; 32],
}

#[derive(Serialize)]
pub struct Output {
    pub msg: String,
}

pub fn service(config: &Config) -> impl HttpServiceFactory {
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
