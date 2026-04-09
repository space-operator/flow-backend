use super::prelude::*;
use crate::{
    db_worker::{
        FindActor, GetUserWorker,
        flow_run_worker::{FlowRunWorker, ForceStopFlow, WaitFinish},
        user_worker::StartDeployment,
    },
    middleware::{
        auth_v1::{AuthEither, AuthenticatedUser, Unverified},
        optional,
        x402::X402Middleware as X402MiddlewareV1,
    },
    read_cache::{CachedRead, ReadCache},
    user::SupabaseAuth,
};
use actix_web::{
    HttpRequest, HttpResponse, Responder,
    http::header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH, LAST_MODIFIED},
};
use anyhow::anyhow;
use flow::flow_registry::ExecutionMode;
use flow::flow_set::{
    DeploymentId, FlowStarter, PreservedBearerToken, StartFlowDeploymentOptions, X402Network,
};
use flow_lib::config::client::FlowRunOrigin;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use value::{Decimal, Value};
use x402_kit::{
    core::Resource,
    facilitator_client::StandardFacilitatorClient,
    networks::svm::assets::{UsdcSolana, UsdcSolanaDevnet},
    paywall::paywall::PayWall,
    schemes::exact_svm::ExactSvm,
    transport::{Accepts, PaymentRequirements},
};

const READ_TIMEOUT: Duration = Duration::from_secs(30);
const READ_TTL: Duration = Duration::from_secs(300);

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
    type Error = uuid::Error;

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

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(try_from = "QueryDe")]
enum Query {
    FlowTag { flow: FlowId, tag: String },
    Id { id: DeploymentId },
}

#[derive(Deserialize)]
struct ReadQuery {
    #[serde(flatten)]
    query: Query,
    inputs: Option<String>,
    #[serde(default)]
    skip_cache: bool,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadBody {
    inputs: Option<ValueSet>,
    #[serde(default)]
    skip_cache: bool,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/read")
        .wrap(config.cors())
        .route(web::get().to(read_deployment_get))
        .route(web::post().to(read_deployment_post))
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
    digits.checked_mul(multiplier).ok_or(anyhow!("overflow"))
}

fn parse_query_inputs(inputs: Option<&str>) -> Result<ValueSet, Error> {
    match inputs {
        Some(inputs) => serde_json::from_str::<ValueSet>(inputs).map_err(|error| {
            Error::custom(
                StatusCode::BAD_REQUEST,
                format!("invalid read inputs query payload: {error}"),
            )
        }),
        None => Ok(ValueSet::new()),
    }
}

fn response_from_cached_read(req: &HttpRequest, cached: CachedRead) -> HttpResponse {
    let last_modified = cached.last_modified();
    let etag_matches = req
        .headers()
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        == Some(cached.etag.as_str());

    let mut response = if etag_matches {
        HttpResponse::NotModified()
    } else {
        HttpResponse::Ok()
    };
    response.insert_header((ETAG, cached.etag));
    response.insert_header((CACHE_CONTROL, cached.cache_control));
    response.insert_header((LAST_MODIFIED, last_modified));

    if etag_matches {
        response.finish()
    } else {
        response.json(cached.body)
    }
}

async fn wait_for_read_output(run_id: FlowRunId, db: &DbPool) -> Result<Value, Error> {
    let db_worker = DBWorker::from_registry();
    let wait = async {
        if let Some(addr) = db_worker
            .send(FindActor::<FlowRunWorker>::new(run_id))
            .await?
        {
            addr.send(WaitFinish)
                .await?
                .map_err(|_| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, "channel closed"))?;
        }
        Ok::<(), Error>(())
    };

    if tokio::time::timeout(READ_TIMEOUT, wait).await.is_err() {
        if let Some(addr) = DBWorker::from_registry()
            .send(FindActor::<FlowRunWorker>::new(run_id))
            .await?
        {
            addr.do_send(ForceStopFlow {
                timeout_millies: 5_000,
                reason: Some("read timeout".to_owned()),
            });
        }
        return Err(Error::custom(StatusCode::REQUEST_TIMEOUT, "read timeout"));
    }

    db.get_admin_conn()
        .await?
        .get_flow_run_output(run_id)
        .await
        .map_err(Into::into)
}

async fn run_read_with_cache<F, Fut>(
    req: &HttpRequest,
    read_cache: &ReadCache,
    request_key: String,
    skip_cache: bool,
    compute: F,
) -> Result<HttpResponse, Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, Error>>,
{
    let cached = if skip_cache {
        read_cache.build_cached_read(compute().await?, READ_TTL)?
    } else {
        read_cache
            .get_or_compute(request_key, READ_TTL, compute)
            .await?
            .0
    };
    Ok(response_from_cached_read(req, cached))
}

async fn load_deployment(
    db: &DbPool,
    starter_user_id: UserId,
    query: &Query,
) -> Result<flow::flow_set::FlowDeployment, Error> {
    let conn = db.get_user_conn(starter_user_id).await?;
    let id = match query {
        Query::FlowTag { flow, tag } => conn.get_deployment_id_from_tag(flow, tag).await?,
        Query::Id { id } => *id,
    };
    let mut deployment = conn.get_deployment(&id).await?;
    let conn = db.get_user_conn(deployment.user_id).await?;
    deployment.flows = conn.get_deployment_flows(&id).await?;
    deployment.wallets_id = deployment
        .flows
        .values()
        .map(|flow| flow.wallets_id())
        .fold(BTreeSet::new(), |mut acc, mut item| {
            acc.append(&mut item);
            acc
        });
    deployment.x402_fees = conn.get_deployment_x402_fees(&id).await?;
    let Some(entrypoint) = deployment.flows.get(&deployment.entrypoint) else {
        return Err(Error::custom(
            StatusCode::INTERNAL_SERVER_ERROR,
            "deployment entrypoint snapshot missing",
        ));
    };
    if !entrypoint.row.read_enabled {
        return Err(Error::custom(StatusCode::FORBIDDEN, "read not allowed"));
    }
    Ok(deployment)
}

fn deployment_target(query: &Query) -> String {
    match query {
        Query::Id { id } => format!("id:{id}"),
        Query::FlowTag { flow, tag } => format!("flow:{flow}:{tag}"),
    }
}

async fn build_paywall(
    conn: &dyn UserConnectionTrait,
    req: &HttpRequest,
    deployment: &flow::flow_set::FlowDeployment,
    x402_1: &X402MiddlewareV1,
) -> Result<Option<PayWall<StandardFacilitatorClient>>, Error> {
    let Some(fees) = deployment.x402_fees.as_ref() else {
        return Ok(None);
    };
    let resource = Resource::builder()
        .url(req.full_url())
        .mime_type("application/json")
        .maybe_output_schema(None)
        .description("read flow deployment")
        .build();
    let pubkeys = conn
        .get_some_wallets(&fees.iter().map(|fee| fee.pay_to).collect::<Vec<_>>())
        .await?
        .into_iter()
        .map(|w| (w.id, flow_lib::solana::Pubkey::new_from_array(w.pubkey)))
        .collect::<BTreeMap<_, _>>();
    let mut accepts = Accepts::new();
    for fee in fees {
        let amount = to_token_amount(fee.amount)
            .map_err(|error| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, error))?;
        let pay_to = pubkeys[&fee.pay_to];
        let requirements: PaymentRequirements = match fee.network {
            X402Network::Base | X402Network::BaseSepolia => {
                return Err(Error::custom(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "network not supported",
                ));
            }
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
        accepts = accepts.push(requirements);
    }
    Ok(Some(
        PayWall::<StandardFacilitatorClient>::builder()
            .facilitator(x402_1.client.clone())
            .resource(resource)
            .accepts(accepts)
            .build(),
    ))
}

async fn start_deployment_read(
    deployment: flow::flow_set::FlowDeployment,
    starter: FlowStarter,
    inputs: ValueSet,
    preserved_bearer_token: Option<PreservedBearerToken>,
    base_url: &str,
) -> Result<FlowRunId, Error> {
    let owner = deployment.user_id;
    let db_worker = DBWorker::from_registry();
    let owner_worker = db_worker
        .send(GetUserWorker {
            user_id: owner,
            base_url: Some(base_url.to_owned()),
        })
        .await?;
    owner_worker
        .send(StartDeployment {
            deployment,
            options: StartFlowDeploymentOptions {
                inputs,
                starter,
                preserved_bearer_token,
                execution_mode: ExecutionMode::ReadSnapshot,
                origin: FlowRunOrigin::Read {},
            },
        })
        .await?
        .map_err(Into::into)
}

async fn read_deployment_impl(
    req: HttpRequest,
    query: Query,
    inputs: ValueSet,
    skip_cache: bool,
    user: AuthEither<AuthenticatedUser, Unverified>,
    sup: web::Data<SupabaseAuth>,
    x402_1: web::Data<X402MiddlewareV1>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> actix_web::Result<HttpResponse> {
    let cache_req = req.clone();
    let preserved_bearer_token = match &user {
        AuthEither::One(user) => {
            user.preserved_bearer_token()
                .as_ref()
                .map(|token| PreservedBearerToken {
                    access_token: token.access_token().clone(),
                    expires_at: *token.expires_at(),
                })
        }
        AuthEither::Two(_) => None,
    };
    let mut starter = match &user {
        AuthEither::One(user) => FlowStarter {
            user_id: *user.user_id(),
            pubkey: flow_lib::solana::Pubkey::new_from_array(*user.pubkey()),
            authenticated: true,
            action_signer: None,
        },
        AuthEither::Two(unverified) => FlowStarter {
            user_id: UserId::nil(),
            pubkey: flow_lib::solana::Pubkey::new_from_array(*unverified.pubkey()),
            authenticated: false,
            action_signer: None,
        },
    };
    let auth_scope = match &user {
        AuthEither::One(user) => format!("user:{}", user.user_id()),
        AuthEither::Two(unverified) => {
            format!("pubkey:{}", bs58::encode(unverified.pubkey()).into_string())
        }
    };

    let deployment = load_deployment(&db, starter.user_id, &query).await?;
    let owner_conn = db.get_user_conn(deployment.user_id).await?;
    let paywall = build_paywall(&*owner_conn, &req, &deployment, &x402_1).await?;
    let target = deployment_target(&query);
    let request_key =
        read_cache.make_request_key("deployment-read", &target, &auth_scope, &inputs)?;

    let handler = async move || {
        if starter.user_id.is_nil() {
            starter.user_id = sup.get_or_create_user(&starter.pubkey.to_bytes()).await?.0;
        }
        run_read_with_cache(
            &cache_req,
            &read_cache,
            request_key,
            skip_cache,
            move || {
                let db = db.clone();
                async move {
                    let run_id = start_deployment_read(
                        deployment,
                        starter,
                        inputs,
                        preserved_bearer_token,
                        &base_url,
                    )
                    .await?;
                    wait_for_read_output(run_id, &db).await
                }
            },
        )
        .await
    };

    match paywall {
        Some(paywall) => {
            let result = paywall
                .handle_payment(req, async move |req| {
                    let resp = handler().await;
                    resp.respond_to(&req).map_into_boxed_body()
                })
                .await;
            match result {
                Ok(resp) => Ok(resp.map_into_boxed_body()),
                Err(error) => Err(error.into()),
            }
        }
        None => Ok(handler().await?.respond_to(&req).map_into_boxed_body()),
    }
}

async fn read_deployment_get(
    req: HttpRequest,
    query: web::Query<ReadQuery>,
    user: AuthEither<AuthenticatedUser, Unverified>,
    sup: web::Data<SupabaseAuth>,
    x402_1: web::Data<X402MiddlewareV1>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    server_base_url: ServerBaseUrl,
) -> actix_web::Result<HttpResponse> {
    let query = query.into_inner();
    let inputs = parse_query_inputs(query.inputs.as_deref())?;
    read_deployment_impl(
        req,
        query.query,
        inputs,
        query.skip_cache,
        user,
        sup,
        x402_1,
        read_cache,
        db,
        server_base_url,
    )
    .await
}

async fn read_deployment_post(
    req: HttpRequest,
    query: web::Query<Query>,
    params: actix_web::Result<web::Json<ReadBody>>,
    user: AuthEither<AuthenticatedUser, Unverified>,
    sup: web::Data<SupabaseAuth>,
    x402_1: web::Data<X402MiddlewareV1>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    server_base_url: ServerBaseUrl,
) -> actix_web::Result<HttpResponse> {
    let params = optional(params)?.map(|params| params.0).unwrap_or_default();
    read_deployment_impl(
        req,
        query.into_inner(),
        params.inputs.unwrap_or_default(),
        params.skip_cache,
        user,
        sup,
        x402_1,
        read_cache,
        db,
        server_base_url,
    )
    .await
}
