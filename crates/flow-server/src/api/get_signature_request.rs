use super::prelude::*;
use crate::db_worker::{
    FindActor, UserWorker,
    flow_run_worker::{FlowRunWorker, SubscribeEvents},
    user_worker::SigReqExists,
};
use db::connection::FlowRunInfo;
use flow_lib::{context::signer::SignatureRequest, flow_run_events::Event};
use futures_util::StreamExt;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    web::resource("/signature_request/{run_id}")
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::get().to(get_signature_request))
}

async fn exists_in_user(
    db_worker: &actix::Addr<DBWorker>,
    req_id: i64,
    user_id: UserId,
) -> Result<bool, Error> {
    let user = db_worker
        .send(FindActor::<UserWorker>::new(user_id))
        .await?;
    Ok(match user {
        Some(user) => user.send(SigReqExists { id: req_id }).await?,
        None => false,
    })
}

#[allow(dead_code)]
async fn exists(
    db_worker: &actix::Addr<DBWorker>,
    req_id: i64,
    run_info: &FlowRunInfo,
) -> Result<bool, Error> {
    if exists_in_user(db_worker, req_id, run_info.user_id).await? {
        return Ok(true);
    }
    for user_id in &run_info.shared_with {
        if exists_in_user(db_worker, req_id, *user_id).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn get_signature_request(
    run_id: web::Path<FlowRunId>,
    auth: web::ReqData<auth::TokenType>,
    db: web::Data<RealDbPool>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
) -> Result<web::Json<SignatureRequest>, Error> {
    let run_id = run_id.into_inner();
    let conn = db.get_admin_conn().await?;
    let run_info = conn.get_flow_run_info(run_id).await?;
    let auth = auth.into_inner();
    if !(auth.flow_run_id() == Some(run_id)
        || auth.user_id().is_some_and(|user_id| {
            run_info.user_id == user_id || run_info.shared_with.contains(&user_id)
        }))
    {
        return Err(Error::custom(StatusCode::NOT_FOUND, "unauthorized"));
    }
    if let Some(flow_run) = db_worker
        .send(FindActor::<FlowRunWorker>::new(run_id))
        .await?
    {
        let (_, mut events) = flow_run
            .send(SubscribeEvents {
                tokens: <_>::from([auth]),
            })
            .await?
            .map_err(|_| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, "channel closed"))?;
        while let Some(event) = events.next().await {
            if let Event::SignatureRequest(req) = event {
                if let Some(req_id) = req.id {
                    if exists(&db_worker, req_id, &run_info).await? {
                        return Ok(web::Json(req));
                    }
                }
            }
        }
    }
    Err(Error::custom(StatusCode::NOT_FOUND, "not found"))
}
