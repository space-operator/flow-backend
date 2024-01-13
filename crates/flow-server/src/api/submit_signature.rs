use super::prelude::*;
use crate::db_worker::user_worker::{SubmitError, SubmitSignature};

#[derive(Deserialize)]
pub struct Params {
    id: i64,
    #[serde(with = "utils::serde_bs58")]
    signature: [u8; 64],
}

#[derive(Serialize)]
pub struct Output {
    success: bool,
}

pub fn service(config: &Config) -> impl HttpServiceFactory {
    web::resource("/submit")
        .wrap(config.cors())
        .route(web::post().to(submit_signature))
}

async fn submit_signature(
    params: web::Json<Params>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<Success>, SubmitError> {
    let params = params.into_inner();

    db_worker
        .send(SubmitSignature {
            id: params.id,
            user_id: UserId::nil(),
            signature: params.signature,
        })
        .await??;

    Ok(web::Json(Success))
}
