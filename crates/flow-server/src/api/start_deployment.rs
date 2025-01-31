use super::prelude::*;
use crate::{
    db_worker::{user_worker::StartDeployment, GetUserWorker},
    middleware::{
        auth_v1::{Auth2, AuthenticatedUser, Unverified},
        optional,
    },
    user::{SignatureAuth, SupabaseAuth},
};
use flow::flow_set::{DeploymentId, FlowStarter, StartFlowDeploymentOptions};
use flow_lib::solana::Pubkey;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Query {
    FlowTag { flow: FlowId, tag: String },
    Id { id: DeploymentId },
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Params {
    inputs: Option<ValueSet>,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
    pub token: String,
}

pub fn service(config: &Config) -> impl HttpServiceFactory {
    web::resource("/start")
        .wrap(config.cors())
        .route(web::post().to(start_deployment))
}

async fn start_deployment(
    query: web::Query<Query>,
    params: actix_web::Result<web::Json<Params>>,
    user: Auth2<AuthenticatedUser, Unverified>,
    db: web::Data<RealDbPool>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
    sup: web::Data<SupabaseAuth>,
    sig: web::Data<SignatureAuth>,
) -> actix_web::Result<web::Json<Output>> {
    let params = optional(params)?;
    let mut starter = match &user {
        Auth2::One(user) => FlowStarter {
            user_id: *user.user_id(),
            pubkey: Pubkey::new_from_array(*user.pubkey()),
            authenticated: true,
        },
        Auth2::Two(unverified) => FlowStarter {
            user_id: UserId::nil(),
            pubkey: Pubkey::new_from_array(*unverified.pubkey()),
            authenticated: false,
        },
    };
    let conn = db.get_user_conn(starter.user_id).await?;
    let id = match query.into_inner() {
        Query::FlowTag { flow, tag } => conn.get_deployment_id_from_tag(&flow, &tag).await?,
        Query::Id { id } => id,
    };
    let mut deployment = conn.get_deployment(&id).await?;

    let conn = db.get_user_conn(deployment.user_id).await?;
    deployment.flows = conn.get_deployment_flows(&id).await?;
    deployment.wallets_id = conn.get_deployment_wallets(&id).await?;

    if starter.user_id.is_nil() {
        starter.user_id = sup.get_or_create_user(&starter.pubkey.to_bytes()).await?.0;
    }

    let owner = deployment.user_id;
    let owner_worker = db_worker
        .send(GetUserWorker { user_id: owner })
        .await
        .map_err(Error::from)?;
    let options = StartFlowDeploymentOptions {
        inputs: params
            .map(|p| p.0.inputs.unwrap_or_default())
            .unwrap_or_default(),
        starter,
    };
    tracing::debug!("{:?}", options);
    let flow_run_id = owner_worker
        .send(StartDeployment {
            deployment,
            options,
        })
        .await
        .map_err(Error::from)??;

    Ok(web::Json(Output {
        flow_run_id,
        token: sig.flow_run_token(flow_run_id),
    }))
}
