use super::prelude::*;
use crate::db_worker::{GetUserWorker, user_worker::StartFlowFresh};
use flow_lib::config::client::PartialConfig;
use hashbrown::HashMap;
use value::Value;

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
    pub partial_config: Option<PartialConfig>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub output_instructions: bool,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/start/{id}")
        .wrap(config.cors())
        .route(web::post().to(start_flow))
}

async fn start_flow(
    flow_id: web::Path<FlowId>,
    params: Option<web::Json<Params>>,
    user: Auth<auth_v1::AuthenticatedUser>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();
    let user_id = *user.user_id();
    let (inputs, partial_config, environment, output_instructions) = params
        .map(
            |web::Json(Params {
                 inputs,
                 partial_config,
                 environment,
                 output_instructions,
             })| (inputs, partial_config, environment, output_instructions),
        )
        .unwrap_or_default();
    let inputs = inputs.into_iter().collect::<ValueSet>();

    if let Some(partial_config) = &partial_config {
        tracing::debug!("partial config: {:?}", partial_config);
    }

    let db_worker = DBWorker::from_registry();
    let flow_run_id = db_worker
        .send(GetUserWorker {
            user_id,
            base_url: Some(base_url),
        })
        .await?
        .send(StartFlowFresh {
            user: flow_lib::User { id: user_id },
            flow_id,
            input: inputs,
            partial_config,
            environment,
            output_instructions,
            action_identity: None,
            action_config: None,
            fees: Vec::new(),
        })
        .await??;

    Ok(web::Json(Output { flow_run_id }))
}
