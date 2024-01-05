use super::prelude::*;
use crate::{
    db_worker::{user_worker::StartFlowShared, GetUserWorker},
    user::{SignatureAuth, SupabaseAuth},
};
use db::pool::DbPool;
use hashbrown::HashMap;
use value::Value;

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
    pub token: String,
}

pub fn service(
    config: &Config,
    db: DbPool,
    sup: web::Data<SupabaseAuth>,
) -> impl HttpServiceFactory {
    web::resource("/start_unverified/{id}")
        .app_data(sup)
        .app_data(web::Data::new(config.signature_auth()))
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(start_flow_unverified))
}

pub async fn find_or_create_user(
    pubkey: &[u8; 32],
    sup: &SupabaseAuth,
    db: &RealDbPool,
) -> Result<UserId, Error> {
    let conn = db.get_admin_conn().await?;
    let user_id = conn
        .get_password(&bs58::encode(&pubkey).into_string())
        .await?
        .map(|pw| pw.user_id);
    let user_id = match user_id {
        Some(user_id) => user_id,
        None => sup.create_user(&pubkey).await?.1,
    };
    Ok(user_id)
}

async fn start_flow_unverified(
    flow_id: web::Path<FlowId>,
    params: Option<web::Json<Params>>,
    user: web::ReqData<auth::Unverified>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
    sup: web::Data<SupabaseAuth>,
    db: web::Data<RealDbPool>,
    sig: web::Data<SignatureAuth>,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();
    let user = user.into_inner();
    let inputs = params
        .map(|web::Json(Params { inputs })| inputs)
        .unwrap_or_default();
    let inputs = inputs.into_iter().collect::<ValueSet>();

    let flow = db
        .get_user_conn(UserId::nil())
        .await?
        .get_flow_info(flow_id)
        .await?;
    if !flow.start_shared && !flow.start_unverified {
        return Err(Error::custom(StatusCode::FORBIDDEN, "not allowed"));
    }

    let user_id = find_or_create_user(&user.pubkey, &sup, &db).await?;

    let starter = db_worker.send(GetUserWorker { user_id }).await?;
    let owner = db_worker
        .send(GetUserWorker {
            user_id: flow.user_id,
        })
        .await?;

    let flow_run_id = owner
        .send(StartFlowShared {
            flow_id,
            input: inputs,
            started_by: (user_id, starter),
        })
        .await??;

    let token = sig.flow_run_token(flow_run_id);

    Ok(web::Json(Output { flow_run_id, token }))
}
