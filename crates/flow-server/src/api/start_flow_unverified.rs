use super::prelude::*;
use crate::{
    db_worker::{GetUserWorker, user_worker::StartFlowShared},
    user::{SignatureAuth, SupabaseAuth},
};
use db::pool::DbPool;
use flow_lib::solana::{Pubkey, SolanaActionConfig};
use hashbrown::HashMap;
use serde_with::{DisplayFromStr, serde_as};
use value::Value;

#[serde_as]
#[derive(Default, Deserialize, Debug)]
pub struct Params {
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
    #[serde(default)]
    pub output_instructions: bool,
    #[serde(default, with = "value::pubkey::opt")]
    pub action_identity: Option<Pubkey>,
    pub action_config: Option<SolanaActionConfig>,
    #[serde(default)]
    #[serde_as(as = "Vec<(DisplayFromStr, _)>")]
    pub fees: Vec<(Pubkey, u64)>,
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
) -> impl HttpServiceFactory + 'static {
    web::resource("/start_unverified/{id}")
        .app_data(sup)
        .app_data(web::Data::new(config.signature_auth()))
        .wrap(config.all_auth(db))
        .wrap(config.cors())
        .route(web::post().to(start_flow_unverified))
}

async fn start_flow_unverified(
    flow_id: web::Path<FlowId>,
    params: Option<web::Json<Params>>,
    user: web::ReqData<auth::Unverified>,

    sup: web::Data<SupabaseAuth>,
    db: web::Data<RealDbPool>,
    sig: web::Data<SignatureAuth>,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();
    let user = user.into_inner();
    let params = params.map(|params| params.0).unwrap_or_default();
    let inputs = params.inputs.into_iter().collect::<ValueSet>();

    let flow = db
        .get_user_conn(UserId::nil())
        .await?
        .get_flow_info(flow_id)
        .await?;
    if !flow.start_shared && !flow.start_unverified {
        return Err(Error::custom(StatusCode::FORBIDDEN, "not allowed"));
    }

    let user_id = sup.get_or_create_user(&user.pubkey).await?.0;

    let db_worker = DBWorker::from_registry();

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
            output_instructions: params.output_instructions,
            action_identity: params.action_identity,
            action_config: params.action_config,
            fees: params.fees,
            started_by: (user_id, starter),
        })
        .await??;

    let token = sig.flow_run_token(flow_run_id);

    Ok(web::Json(Output { flow_run_id, token }))
}
