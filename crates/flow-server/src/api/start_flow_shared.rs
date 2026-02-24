use super::prelude::*;
use crate::db_worker::{GetUserWorker, user_worker::StartFlowShared};
use db::pool::DbPool;
use hashbrown::HashMap;
use value::Value;

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
    #[serde(default)]
    pub output_instructions: bool,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/start_shared/{id}")
        .wrap(config.cors())
        .route(web::post().to(start_flow_shared))
}

async fn start_flow_shared(
    flow_id: web::Path<FlowId>,
    params: Option<web::Json<Params>>,
    user: Auth<auth_v1::AuthenticatedUser>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();
    let (inputs, output_instructions) = params
        .map(
            |web::Json(Params {
                 inputs,
                 output_instructions,
             })| (inputs, output_instructions),
        )
        .unwrap_or_default();
    let inputs = inputs.into_iter().collect::<ValueSet>();
    let user_id = *user.user_id();
    let flow = db
        .get_user_conn(user_id)
        .await?
        .get_flow_info(flow_id)
        .await?;
    if !flow.start_shared {
        return Err(Error::custom(StatusCode::FORBIDDEN, "not allowed"));
    }

    let db_worker = DBWorker::from_registry();

    let starter = db_worker
        .send(GetUserWorker {
            user_id,
            base_url: Some(base_url.clone()),
        })
        .await?;
    let owner = db_worker
        .send(GetUserWorker {
            user_id: flow.user_id,
            base_url: Some(base_url.clone()),
        })
        .await?;

    let flow_run_id = owner
        .send(StartFlowShared {
            flow_id,
            input: inputs,
            partial_config: None,
            environment: <_>::default(),
            output_instructions,
            action_identity: None,
            action_config: None,
            fees: Vec::new(),
            started_by: (user_id, starter),
        })
        .await??;

    Ok(web::Json(Output { flow_run_id }))
}
