use super::prelude::*;
use crate::{
    db_worker::{GetUserWorker, user_worker::StartDeployment},
    middleware::{
        auth_v1::{AuthEither, AuthenticatedUser, Unverified},
        optional,
    },
    user::{SignatureAuth, SupabaseAuth},
};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header::HeaderMap,
    middleware::{self, Next},
};
use flow::flow_set::{DeploymentId, FlowStarter, StartFlowDeploymentOptions, X402Network};
use flow_lib::solana::Pubkey;
use serde_with::serde_as;
use std::{collections::BTreeMap, num::ParseIntError};
use value::with::AsPubkey;
use x402_actix::{
    facilitator_client::FacilitatorClient, middleware::X402Middleware, price::IntoPriceTag,
};
use x402_rs::{
    network::USDCDeployment,
    types::{MixedAddress, MoneyAmount},
};

fn default_tag() -> String {
    "latest".to_owned()
}

#[derive(Deserialize)]
#[serde(untagged)]
enum QueryDe {
    FlowTag {
        flow: String,
        #[serde(default = "default_tag")]
        tag: String,
    },
    Id {
        id: DeploymentId,
    },
}

impl TryFrom<QueryDe> for Query {
    type Error = ParseIntError;

    fn try_from(value: QueryDe) -> Result<Self, Self::Error> {
        Ok(match value {
            QueryDe::FlowTag { flow, tag } => Query::FlowTag {
                flow: flow.parse()?,
                tag,
            },
            QueryDe::Id { id } => Query::Id { id },
        })
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(try_from = "QueryDe")]
pub enum Query {
    FlowTag { flow: i32, tag: String },
    Id { id: DeploymentId },
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Params {
    inputs: Option<ValueSet>,
    #[serde_as(as = "Option<AsPubkey>")]
    action_signer: Option<Pubkey>,
}

#[derive(Serialize)]
pub struct Output {
    pub flow_run_id: FlowRunId,
    pub token: String,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/start")
        .wrap(middleware::from_fn(log_full))
        .wrap(config.cors())
        .route(web::post().to(start_deployment))
}

fn to_network(net: X402Network) -> x402_rs::network::Network {
    use x402_rs::network::Network;
    match net {
        X402Network::Base => Network::Base,
        X402Network::BaseSepolia => Network::BaseSepolia,
        X402Network::Solana => Network::Solana,
        X402Network::SolanaDevnet => Network::SolanaDevnet,
    }
}

#[allow(dead_code)]
fn pretty_print(map: &HeaderMap) -> String {
    map.iter()
        .map(|(k, v)| {
            format!(
                "{}: {}\n",
                k.as_str(),
                String::from_utf8_lossy(v.as_bytes())
            )
        })
        .collect()
}

#[allow(dead_code)]
async fn log_full(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    tracing::debug!("{}", pretty_print(req.headers()));
    next.call(req).await
}

async fn start_deployment(
    query: web::Query<Query>,
    params: actix_web::Result<web::Json<Params>>,
    user: AuthEither<AuthenticatedUser, Unverified>,
    db: web::Data<DbPool>,
    sup: web::Data<SupabaseAuth>,
    sig: web::Data<SignatureAuth>,
    x402: web::Data<X402Middleware<FacilitatorClient>>,
    req: actix_web::HttpRequest,
) -> actix_web::Result<web::Json<Output>> {
    // tracing::debug!("{}", pretty_print(req.headers()));

    let params = optional(params)?.map(|x| x.0);
    let (action_signer, inputs) = match params {
        Some(params) => (params.action_signer, params.inputs.unwrap_or_default()),
        None => (None, Default::default()),
    };
    let mut starter = match &user {
        AuthEither::One(user) => FlowStarter {
            user_id: *user.user_id(),
            pubkey: Pubkey::new_from_array(*user.pubkey()),
            authenticated: true,
            action_signer,
        },
        AuthEither::Two(unverified) => FlowStarter {
            user_id: UserId::nil(),
            pubkey: Pubkey::new_from_array(*unverified.pubkey()),
            authenticated: false,
            action_signer,
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
    deployment.x402_fees = conn.get_deployment_x402_fees(&id).await?;

    if let Some(fees) = deployment.x402_fees.as_ref() {
        let pubkeys = conn
            .get_some_wallets(&fees.iter().map(|fee| fee.pay_to).collect::<Vec<_>>())
            .await?
            .into_iter()
            .map(|w| (w.id, Pubkey::new_from_array(w.pubkey)))
            .collect::<BTreeMap<_, _>>();
        let mut paygate = x402.with_mime_type("application/json");
        for fee in fees {
            paygate = paygate.or_price_tag(
                USDCDeployment::by_network(to_network(fee.network))
                    .amount(MoneyAmount(fee.amount))
                    .pay_to(MixedAddress::Solana(pubkeys[&fee.pay_to]))
                    .build()
                    .unwrap(),
            );
        }
        let paygate = paygate.to_paygate(&req.full_url());
        let payload = paygate.extract_payment_payload(req.headers()).await?;
        let r = paygate.verify_payment(payload).await?;
        paygate.settle_payment(&r).await?;
    }

    if starter.user_id.is_nil() {
        starter.user_id = sup.get_or_create_user(&starter.pubkey.to_bytes()).await?.0;
    }

    let options = StartFlowDeploymentOptions { inputs, starter };

    let owner = deployment.user_id;
    let db_worker = DBWorker::from_registry();
    let owner_worker = db_worker
        .send(GetUserWorker { user_id: owner })
        .await
        .map_err(Error::from)?;
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

#[cfg(test)]
mod tests {
    use actix_web::{FromRequest, test::TestRequest};

    use super::*;

    #[actix_web::test]
    async fn test_query() {
        let req = TestRequest::with_uri("/start?flow=123&tag=latest").to_http_request();
        let query = web::Query::<Query>::extract(&req).await.unwrap();
        assert_eq!(
            query.0,
            Query::FlowTag {
                flow: 123,
                tag: "latest".to_owned()
            }
        );

        let req = TestRequest::with_uri("/start?flow=123").to_http_request();
        let query = web::Query::<Query>::extract(&req).await.unwrap();
        assert_eq!(
            query.0,
            Query::FlowTag {
                flow: 123,
                tag: "latest".to_owned()
            }
        );
    }
}
