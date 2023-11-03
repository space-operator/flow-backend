use crate::db_worker::{
    user_worker::{SubmitError, SubmitSignature},
    FindActor, UserWorker,
};

use super::prelude::*;

#[derive(Deserialize)]
pub struct Params {
    id: i64,
    #[serde(deserialize_with = "super::bs58_decode")]
    signature: [u8; 64],
}

#[derive(Serialize)]
pub struct Output {
    success: bool,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    web::resource("/submit")
        .wrap(config.apikey_auth(db))
        .wrap(config.cors())
        .route(web::post().to(submit_signature))
}

async fn submit_signature(
    params: web::Json<Params>,
    user: web::ReqData<auth::JWTPayload>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<Output>, SubmitError> {
    let user = user.into_inner();
    let params = params.into_inner();

    db_worker
        .send(FindActor::<UserWorker>::new(user.user_id))
        .await?
        .ok_or(SubmitError::NotFound)?
        .send(SubmitSignature {
            id: params.id,
            user_id: user.user_id,
            signature: params.signature,
        })
        .await??;

    Ok(web::Json(Output { success: true }))
}
