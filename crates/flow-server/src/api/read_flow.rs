use super::prelude::*;
use crate::{
    db_worker::{
        FindActor, GetUserWorker,
        flow_run_worker::{FlowRunWorker, ForceStopFlow, WaitFinish},
        user_worker::{StartFlowFresh, StartFlowShared},
    },
    middleware::auth_v1::Unverified,
    read_cache::{CachedRead, ReadCache},
    user::SupabaseAuth,
};
use actix_web::{
    HttpRequest, HttpResponse,
    http::header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH, LAST_MODIFIED},
};
use db::connection::DbClient;
use flow::flow_registry::ExecutionMode;
use flow_lib::config::client::FlowRunOrigin;
use hashbrown::{HashMap, HashSet};
use std::time::Duration;
use value::Value;

const READ_TIMEOUT: Duration = Duration::from_secs(30);
const READ_TTL: Duration = Duration::from_secs(60);

#[derive(Default, Deserialize)]
struct ReadQuery {
    inputs: Option<String>,
    #[serde(default)]
    skip_cache: bool,
}

#[derive(Default, Deserialize)]
struct ReadBody {
    #[serde(default)]
    inputs: HashMap<String, Value>,
    #[serde(default)]
    skip_cache: bool,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/read/{id}")
        .wrap(config.cors())
        .route(web::get().to(read_flow_get))
        .route(web::post().to(read_flow_post))
}

pub fn service_shared(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/read_shared/{id}")
        .wrap(config.cors())
        .route(web::get().to(read_flow_shared_get))
        .route(web::post().to(read_flow_shared_post))
}

pub fn service_unverified(
    config: &Config,
    sup: web::Data<SupabaseAuth>,
) -> impl HttpServiceFactory + 'static {
    web::resource("/read_unverified/{id}")
        .app_data(sup)
        .app_data(web::Data::new(config.signature_auth()))
        .wrap(config.cors())
        .route(web::get().to(read_flow_unverified_get))
        .route(web::post().to(read_flow_unverified_post))
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

async fn start_owner_or_adapter_read(
    flow_id: FlowId,
    inputs: ValueSet,
    user_id: UserId,
    user_pubkey: [u8; 32],
    preserved_bearer_token: Option<flow::flow_set::PreservedBearerToken>,
    db: DbPool,
    base_url: String,
) -> Result<FlowRunId, Error> {
    let flow_owner_id = {
        let conn = db.get_conn().await?;
        let row = conn
            .do_query_opt(
                "SELECT user_id, read_enabled FROM flows_v2 WHERE uuid = $1",
                &[&flow_id],
            )
            .await
            .map_err(DbError::exec("get flow owner"))?
            .ok_or_else(|| Error::custom(StatusCode::NOT_FOUND, "not found"))?;
        let read_enabled: bool = row
            .try_get("read_enabled")
            .map_err(DbError::data("flows_v2.read_enabled"))?;
        if !read_enabled {
            return Err(Error::custom(StatusCode::FORBIDDEN, "read not allowed"));
        }
        row.try_get("user_id")
            .map_err(DbError::data("flows_v2.user_id"))?
    };

    let db_worker = DBWorker::from_registry();
    if flow_owner_id == user_id {
        return db_worker
            .send(GetUserWorker {
                user_id,
                base_url: Some(base_url.clone()),
            })
            .await?
            .send(StartFlowFresh {
                user: flow_lib::User { id: user_id },
                flow_id,
                input: inputs,
                preserved_bearer_token,
                execution_mode: ExecutionMode::ReadSnapshot,
                origin: FlowRunOrigin::Read {},
                partial_config: None,
                environment: <_>::default(),
                output_instructions: false,
                action_identity: None,
                action_config: None,
                fees: Vec::new(),
            })
            .await?
            .map_err(Into::into);
    }

    let has_adapter_access = {
        let owner_wallets = db.get_user_conn(flow_owner_id).await?.get_wallets().await?;
        let owner_adapter_pubkeys = owner_wallets
            .into_iter()
            .filter_map(|wallet| (wallet.keypair.is_none()).then_some(wallet.pubkey))
            .collect::<HashSet<_>>();

        if owner_adapter_pubkeys.contains(&user_pubkey) {
            true
        } else {
            let starter_wallets = db.get_user_conn(user_id).await?.get_wallets().await?;
            starter_wallets
                .into_iter()
                .filter(|wallet| wallet.keypair.is_none())
                .any(|wallet| owner_adapter_pubkeys.contains(&wallet.pubkey))
        }
    };
    if !has_adapter_access {
        return Err(Error::custom(StatusCode::NOT_FOUND, "not found"));
    }

    let starter = db_worker
        .send(GetUserWorker {
            user_id,
            base_url: Some(base_url.clone()),
        })
        .await?;
    let owner = db_worker
        .send(GetUserWorker {
            user_id: flow_owner_id,
            base_url: Some(base_url),
        })
        .await?;

    owner
        .send(StartFlowShared {
            flow_id,
            input: inputs,
            preserved_bearer_token,
            execution_mode: ExecutionMode::ReadSnapshot,
            origin: FlowRunOrigin::Read {},
            partial_config: None,
            environment: <_>::default(),
            output_instructions: false,
            action_identity: None,
            action_config: None,
            fees: Vec::new(),
            started_by: (user_id, starter),
        })
        .await?
        .map_err(Into::into)
}

async fn read_flow_common(
    req: HttpRequest,
    flow_id: FlowId,
    inputs: ValueSet,
    skip_cache: bool,
    auth_scope: String,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    start: impl FnOnce()
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FlowRunId, Error>>>>,
) -> Result<HttpResponse, Error> {
    let request_key =
        read_cache.make_request_key("flow-read", &flow_id.to_string(), &auth_scope, &inputs)?;
    run_read_with_cache(&req, &read_cache, request_key, skip_cache, move || {
        let db = db.clone();
        Box::pin(async move {
            let run_id = start().await?;
            wait_for_read_output(run_id, &db).await
        })
    })
    .await
}

async fn read_flow_get(
    req: HttpRequest,
    flow_id: web::Path<FlowId>,
    query: web::Query<ReadQuery>,
    user: Auth<auth_v1::AuthenticatedUser>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<HttpResponse, Error> {
    let flow_id = flow_id.into_inner();
    let query = query.into_inner();
    let inputs = parse_query_inputs(query.inputs.as_deref())?;
    let start_inputs = inputs.clone();
    let db_pool = db.get_ref().clone();
    let base_url = base_url.clone();
    let user_id = *user.user_id();
    let user_pubkey = *user.pubkey();
    let preserved_bearer_token =
        user.preserved_bearer_token()
            .as_ref()
            .map(|token| flow::flow_set::PreservedBearerToken {
                access_token: token.access_token().clone(),
                expires_at: *token.expires_at(),
            });
    let auth_scope = format!("user:{}", user.user_id());
    read_flow_common(
        req,
        flow_id,
        inputs,
        query.skip_cache,
        auth_scope,
        read_cache,
        db.clone(),
        move || {
            Box::pin(start_owner_or_adapter_read(
                flow_id,
                start_inputs,
                user_id,
                user_pubkey,
                preserved_bearer_token,
                db_pool,
                base_url,
            ))
        },
    )
    .await
}

async fn read_flow_post(
    req: HttpRequest,
    flow_id: web::Path<FlowId>,
    body: Option<web::Json<ReadBody>>,
    user: Auth<auth_v1::AuthenticatedUser>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<HttpResponse, Error> {
    let flow_id = flow_id.into_inner();
    let body = body.map(|body| body.0).unwrap_or_default();
    let inputs = body.inputs.into_iter().collect::<ValueSet>();
    let start_inputs = inputs.clone();
    let db_pool = db.get_ref().clone();
    let base_url = base_url.clone();
    let user_id = *user.user_id();
    let user_pubkey = *user.pubkey();
    let preserved_bearer_token =
        user.preserved_bearer_token()
            .as_ref()
            .map(|token| flow::flow_set::PreservedBearerToken {
                access_token: token.access_token().clone(),
                expires_at: *token.expires_at(),
            });
    let auth_scope = format!("user:{}", user.user_id());
    read_flow_common(
        req,
        flow_id,
        inputs,
        body.skip_cache,
        auth_scope,
        read_cache,
        db.clone(),
        move || {
            Box::pin(start_owner_or_adapter_read(
                flow_id,
                start_inputs,
                user_id,
                user_pubkey,
                preserved_bearer_token,
                db_pool,
                base_url,
            ))
        },
    )
    .await
}

async fn start_shared_read(
    flow_id: FlowId,
    inputs: ValueSet,
    user_id: UserId,
    preserved_bearer_token: Option<flow::flow_set::PreservedBearerToken>,
    db: DbPool,
    base_url: String,
) -> Result<FlowRunId, Error> {
    let flow = db
        .get_user_conn(user_id)
        .await?
        .get_flow_info(flow_id)
        .await?;
    if !flow.read_enabled {
        return Err(Error::custom(StatusCode::FORBIDDEN, "read not allowed"));
    }
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
            base_url: Some(base_url),
        })
        .await?;

    owner
        .send(StartFlowShared {
            flow_id,
            input: inputs,
            preserved_bearer_token,
            execution_mode: ExecutionMode::ReadSnapshot,
            origin: FlowRunOrigin::Read {},
            partial_config: None,
            environment: <_>::default(),
            output_instructions: false,
            action_identity: None,
            action_config: None,
            fees: Vec::new(),
            started_by: (user_id, starter),
        })
        .await?
        .map_err(Into::into)
}

async fn read_flow_shared_get(
    req: HttpRequest,
    flow_id: web::Path<FlowId>,
    query: web::Query<ReadQuery>,
    user: Auth<auth_v1::AuthenticatedUser>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<HttpResponse, Error> {
    let flow_id = flow_id.into_inner();
    let query = query.into_inner();
    let inputs = parse_query_inputs(query.inputs.as_deref())?;
    let start_inputs = inputs.clone();
    let db_pool = db.get_ref().clone();
    let base_url = base_url.clone();
    let user_id = *user.user_id();
    let preserved_bearer_token =
        user.preserved_bearer_token()
            .as_ref()
            .map(|token| flow::flow_set::PreservedBearerToken {
                access_token: token.access_token().clone(),
                expires_at: *token.expires_at(),
            });
    let auth_scope = format!("user:{}", user.user_id());
    read_flow_common(
        req,
        flow_id,
        inputs,
        query.skip_cache,
        auth_scope,
        read_cache,
        db.clone(),
        move || {
            Box::pin(start_shared_read(
                flow_id,
                start_inputs,
                user_id,
                preserved_bearer_token,
                db_pool,
                base_url,
            ))
        },
    )
    .await
}

async fn read_flow_shared_post(
    req: HttpRequest,
    flow_id: web::Path<FlowId>,
    body: Option<web::Json<ReadBody>>,
    user: Auth<auth_v1::AuthenticatedUser>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<HttpResponse, Error> {
    let flow_id = flow_id.into_inner();
    let body = body.map(|body| body.0).unwrap_or_default();
    let inputs = body.inputs.into_iter().collect::<ValueSet>();
    let start_inputs = inputs.clone();
    let db_pool = db.get_ref().clone();
    let base_url = base_url.clone();
    let user_id = *user.user_id();
    let preserved_bearer_token =
        user.preserved_bearer_token()
            .as_ref()
            .map(|token| flow::flow_set::PreservedBearerToken {
                access_token: token.access_token().clone(),
                expires_at: *token.expires_at(),
            });
    let auth_scope = format!("user:{}", user.user_id());
    read_flow_common(
        req,
        flow_id,
        inputs,
        body.skip_cache,
        auth_scope,
        read_cache,
        db.clone(),
        move || {
            Box::pin(start_shared_read(
                flow_id,
                start_inputs,
                user_id,
                preserved_bearer_token,
                db_pool,
                base_url,
            ))
        },
    )
    .await
}

async fn start_unverified_read(
    flow_id: FlowId,
    inputs: ValueSet,
    user_pubkey: [u8; 32],
    sup: SupabaseAuth,
    db: DbPool,
    base_url: String,
) -> Result<FlowRunId, Error> {
    let flow = db
        .get_user_conn(UserId::nil())
        .await?
        .get_flow_info(flow_id)
        .await?;
    if !flow.read_enabled {
        return Err(Error::custom(StatusCode::FORBIDDEN, "read not allowed"));
    }
    if !flow.start_shared && !flow.start_unverified {
        return Err(Error::custom(StatusCode::FORBIDDEN, "not allowed"));
    }

    let user_id = sup.get_or_create_user(&user_pubkey).await?.0;
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
            base_url: Some(base_url),
        })
        .await?;

    owner
        .send(StartFlowShared {
            flow_id,
            input: inputs,
            preserved_bearer_token: None,
            execution_mode: ExecutionMode::ReadSnapshot,
            origin: FlowRunOrigin::Read {},
            partial_config: None,
            environment: <_>::default(),
            output_instructions: false,
            action_identity: None,
            action_config: None,
            fees: Vec::new(),
            started_by: (user_id, starter),
        })
        .await?
        .map_err(Into::into)
}

async fn read_flow_unverified_get(
    req: HttpRequest,
    flow_id: web::Path<FlowId>,
    query: web::Query<ReadQuery>,
    user: Auth<Unverified>,
    sup: web::Data<SupabaseAuth>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<HttpResponse, Error> {
    let flow_id = flow_id.into_inner();
    let query = query.into_inner();
    let inputs = parse_query_inputs(query.inputs.as_deref())?;
    let start_inputs = inputs.clone();
    let db_pool = db.get_ref().clone();
    let base_url = base_url.clone();
    let sup = sup.get_ref().clone();
    let user_pubkey = *user.pubkey();
    let auth_scope = format!("pubkey:{}", bs58::encode(user.pubkey()).into_string());
    read_flow_common(
        req,
        flow_id,
        inputs,
        query.skip_cache,
        auth_scope,
        read_cache,
        db.clone(),
        move || {
            Box::pin(start_unverified_read(
                flow_id,
                start_inputs,
                user_pubkey,
                sup,
                db_pool,
                base_url,
            ))
        },
    )
    .await
}

async fn read_flow_unverified_post(
    req: HttpRequest,
    flow_id: web::Path<FlowId>,
    body: Option<web::Json<ReadBody>>,
    user: Auth<Unverified>,
    sup: web::Data<SupabaseAuth>,
    read_cache: web::Data<ReadCache>,
    db: web::Data<DbPool>,
    ServerBaseUrl(base_url): ServerBaseUrl,
) -> Result<HttpResponse, Error> {
    let flow_id = flow_id.into_inner();
    let body = body.map(|body| body.0).unwrap_or_default();
    let inputs = body.inputs.into_iter().collect::<ValueSet>();
    let start_inputs = inputs.clone();
    let db_pool = db.get_ref().clone();
    let base_url = base_url.clone();
    let sup = sup.get_ref().clone();
    let user_pubkey = *user.pubkey();
    let auth_scope = format!("pubkey:{}", bs58::encode(user.pubkey()).into_string());
    read_flow_common(
        req,
        flow_id,
        inputs,
        body.skip_cache,
        auth_scope,
        read_cache,
        db.clone(),
        move || {
            Box::pin(start_unverified_read(
                flow_id,
                start_inputs,
                user_pubkey,
                sup,
                db_pool,
                base_url,
            ))
        },
    )
    .await
}
