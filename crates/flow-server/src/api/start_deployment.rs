use super::prelude::*;
use crate::{
    db_worker::{GetUserWorker, user_worker::StartDeployment},
    middleware::{
        auth_v1::{AuthEither, AuthenticatedUser, Unverified},
        optional,
        x402::X402Middleware as X402MiddlewareV1,
    },
    user::{SignatureAuth, SupabaseAuth},
};
use actix_web::{
    HttpResponse, Responder,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header::HeaderMap,
    middleware::Next,
};
use anyhow::anyhow;
use flow::flow_set::{DeploymentId, FlowStarter, StartFlowDeploymentOptions, X402Network};
use flow_lib::solana::Pubkey;
use serde_with::serde_as;
use std::{collections::BTreeMap, num::ParseIntError};
use value::{Decimal, with::AsPubkey};
use x402_kit::{
    core::Resource,
    facilitator_client::StandardFacilitatorClient,
    networks::svm::assets::{UsdcSolana, UsdcSolanaDevnet},
    paywall::paywall::PayWall,
    schemes::exact_svm::ExactSvm,
    transport::{Accepts, PaymentRequirements},
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
        // .wrap(middleware::from_fn(log_full))
        .wrap(config.cors())
        .route(web::post().to(start_deployment))
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

fn to_token_amount(money_amount: Decimal) -> Result<u64, anyhow::Error> {
    let token_decimals = 6;
    let money_decimals = money_amount.scale();
    if money_decimals > token_decimals {
        return Err(anyhow!(
            "Too big of a precision: {} vs {} on token",
            money_decimals,
            token_decimals
        ));
    }
    let scale_diff = token_decimals - money_decimals;
    let multiplier = 10u64.pow(scale_diff);
    let digits =
        u64::try_from(money_amount.mantissa()).map_err(|_| anyhow!("amount is negative"))?;
    let value = digits.checked_mul(multiplier).ok_or(anyhow!("overflow"))?;
    Ok(value)
}

async fn start_deployment(
    query: web::Query<Query>,
    params: actix_web::Result<web::Json<Params>>,
    user: AuthEither<AuthenticatedUser, Unverified>,
    db: web::Data<DbPool>,
    sup: web::Data<SupabaseAuth>,
    sig: web::Data<SignatureAuth>,
    x402_1: web::Data<X402MiddlewareV1>,
    req: actix_web::HttpRequest,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> actix_web::Result<HttpResponse<impl MessageBody>> {
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

    let paywall = if let Some(fees) = deployment.x402_fees.as_ref() {
        let resource = Resource::builder()
            .url(req.full_url())
            .mime_type("application/json")
            .maybe_output_schema(None)
            .description("start flow deployment")
            .build();
        let pubkeys = conn
            .get_some_wallets(&fees.iter().map(|fee| fee.pay_to).collect::<Vec<_>>())
            .await?
            .into_iter()
            .map(|w| (w.id, Pubkey::new_from_array(w.pubkey)))
            .collect::<BTreeMap<_, _>>();
        let mut accepts = Accepts::new();
        for fee in fees {
            let amount = to_token_amount(fee.amount)
                .map_err(|error| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, error))?;
            let pay_to = pubkeys[&fee.pay_to];
            let r: PaymentRequirements = match fee.network {
                X402Network::Base => return Err(Error::custom(StatusCode::INTERNAL_SERVER_ERROR, "network not supported").into()),
                X402Network::BaseSepolia => return Err(Error::custom(StatusCode::INTERNAL_SERVER_ERROR, "network not supported").into()),
                X402Network::Solana => ExactSvm::builder()
                    .amount(amount)
                    .asset(UsdcSolana)
                    .pay_to(pay_to)
                    .build()
                    .into(),
                X402Network::SolanaDevnet => ExactSvm::builder()
                    .amount(amount)
                    .asset(UsdcSolanaDevnet)
                    .pay_to(pay_to)
                    .build()
                    .into(),
            };
            accepts = accepts.push(r);
        }
        let paywall = PayWall::<StandardFacilitatorClient>::builder()
            .facilitator(x402_1.client.clone())
            .resource(resource)
            .accepts(accepts)
            .build();
        Some(paywall)
    } else {
        None
    };

    let handler = async move || {
        if starter.user_id.is_nil() {
            starter.user_id = sup.get_or_create_user(&starter.pubkey.to_bytes()).await?.0;
        }

        let options = StartFlowDeploymentOptions { inputs, starter };

        let owner = deployment.user_id;
        let db_worker = DBWorker::from_registry();
        let owner_worker = db_worker
            .send(GetUserWorker {
                user_id: owner,
                base_url: Some(base_url.clone()),
            })
            .await
            .map_err(Error::from)?;
        let flow_run_id = owner_worker
            .send(StartDeployment {
                deployment,
                options,
            })
            .await
            .map_err(Error::from)??;

        Ok::<_, actix_web::Error>(web::Json(Output {
            flow_run_id,
            token: sig.flow_run_token(flow_run_id),
        }))
    };

    match paywall {
        Some(paywall) => {
            let result = paywall
                .handle_payment(req, async move |req| {
                    let resp = handler().await;
                    let resp = resp.respond_to(&req);
                    resp
                })
                .await;
            match result {
                Ok(resp) => Ok(resp),
                Err(error) => {
                    return Err(error.into());
                }
            }
        }
        None => Ok(handler().await.respond_to(&req)),
    }
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
