use super::prelude::*;
use crate::db_worker::user_worker::{SubmitError, SubmitSignature};
use bytes::Bytes;
use serde_with::{base64::Base64, serde_as};

#[serde_as]
#[derive(Deserialize)]
pub struct Params {
    id: i64,
    #[serde(with = "utils::serde_bs58")]
    signature: [u8; 64],
    #[serde_as(as = "Option<Base64>")]
    new_msg: Option<Bytes>,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/submit")
        .wrap(config.cors())
        .route(web::post().to(submit_signature))
}

async fn submit_signature(params: web::Json<Params>) -> Result<web::Json<Success>, SubmitError> {
    let params = params.into_inner();

    let db_worker = DBWorker::from_registry();
    db_worker
        .send(SubmitSignature {
            id: params.id,
            user_id: UserId::nil(),
            signature: params.signature,
            new_msg: params.new_msg,
        })
        .await??;

    Ok(web::Json(Success))
}
