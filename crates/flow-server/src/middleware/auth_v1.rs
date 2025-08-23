use crate::{
    error::ErrorBody,
    user::{FLOW_RUN_TOKEN_PREFIX, SignatureAuth},
};
use actix_web::{
    FromRequest, HttpRequest, ResponseError,
    http::{
        StatusCode,
        header::{AUTHORIZATION, HeaderName},
    },
    web::{self, ServiceConfig},
};
use chrono::Utc;
use db::{apikey, pool::DbPool};
use flow_lib::{FlowRunId, UserId};
use futures_util::{FutureExt, future::LocalBoxFuture};
use getset::Getters;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::{
    future::Future,
    ops::{ControlFlow, Deref},
};
use thiserror::Error as ThisError;

macro_rules! early_return {
    ($t:expr) => {{
        let c: ControlFlow<_, _> = $t;
        if let ControlFlow::Break(b) = c {
            return b;
        }
    }};
}

fn control_flow<T>(r: Result<T, AuthError>) -> ControlFlow<Result<T, AuthError>> {
    if matches!(&r, Err(e) if e.try_again()) {
        ControlFlow::Continue(())
    } else {
        ControlFlow::Break(r)
    }
}

fn rsplit(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let dot = b.iter().rposition(|c| *c == b'.')?;
    Some((&b[..dot], &b[dot + 1..]))
}

trait Identity: Sized {
    fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthV1>,
    ) -> impl Future<Output = Result<Self, AuthError>> + 'a;
}

#[derive(Deserialize)]
struct Payload<'a> {
    exp: i64,
    #[allow(dead_code)]
    role: Role,
    sub: UserId,
    #[serde(borrow)]
    user_metadata: UserMetadata<'a>,
}

#[derive(Deserialize)]
struct UserMetadata<'a> {
    pub_key: &'a str,
}

#[derive(Deserialize)]
enum Role {
    #[serde(rename = "authenticated")]
    Authenticated,
}

#[derive(ThisError, Debug)]
#[error("unauthenticated")]
pub enum AuthError {
    NotConfigured,
    NoHeader(HeaderName),
    InvalidFormat,
    HmacFailed,
    InvalidPayload,
    Expired,
    Db(#[from] db::Error),
}

impl AuthError {
    fn try_again(&self) -> bool {
        match self {
            AuthError::NotConfigured => false,
            AuthError::NoHeader(_) => true,
            AuthError::InvalidFormat => true,
            AuthError::HmacFailed => false,
            AuthError::InvalidPayload => false,
            AuthError::Expired => false,
            AuthError::Db(_) => false,
        }
    }
}

impl ResponseError for AuthError {
    fn status_code(&self) -> StatusCode {
        match self {
            AuthError::NotConfigured | AuthError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        ErrorBody::build(self)
    }
}

pub fn configure(cfg: &mut ServiceConfig, server_config: &crate::Config, db: &DbPool) {
    let state = match AuthV1::new(server_config, db) {
        Ok(state) => state,
        Err(error) => {
            tracing::error!("could not build auth_v1 middleware: {}", error);
            return;
        }
    };
    cfg.app_data(web::ThinData(state));
}

#[derive(Clone)]
pub struct AuthV1 {
    hmac: Hmac<Sha256>,
    pool: DbPool,
    sig: SignatureAuth,
}

#[derive(ThisError, Debug)]
#[error("supabase.jwt_key not found in config")]
pub struct ConfigError;

impl AuthV1 {
    pub fn new(server_config: &crate::Config, db: &DbPool) -> Result<Self, ConfigError> {
        let Some(ref jwt_key) = server_config.supabase.jwt_key else {
            return Err(ConfigError);
        };
        let hmac = Hmac::new_from_slice(jwt_key.as_bytes()).unwrap();
        let sig = SignatureAuth::new(server_config.blake3_key);
        Ok(AuthV1 {
            hmac,
            pool: db.clone(),
            sig,
        })
    }

    pub async fn ws_authenticate(
        &self,
        token: &str,
    ) -> Result<AuthEither<AuthenticatedUser, FlowRunToken>, AuthError> {
        if token.starts_with(apikey::KEY_PREFIX) {
            let key = apikey_verify_inner(token, self).await?;
            Ok(AuthEither::One(AuthenticatedUser {
                user_id: key.user_id,
                pubkey: key.pubkey,
            }))
        } else if token.starts_with(FLOW_RUN_TOKEN_PREFIX) {
            Ok(AuthEither::Two(FlowRunToken {
                id: flow_run_token_verify_inner(token, self)?.id,
            }))
        } else {
            let jwt = jwt_verify_inner(token.as_bytes(), self)?;
            Ok(AuthEither::One(AuthenticatedUser {
                user_id: jwt.user_id,
                pubkey: jwt.pubkey,
            }))
        }
    }
}

#[derive(Getters)]
pub struct Jwt {
    #[get = "pub"]
    token: String,
    #[get = "pub"]
    user_id: UserId,
    #[get = "pub"]
    pubkey: [u8; 32],
}

fn jwt_verify_inner(token: &[u8], auth: &AuthV1) -> Result<Jwt, AuthError> {
    let token_str = String::from_utf8(token.to_owned()).map_err(|_| AuthError::InvalidFormat)?;
    let now = Utc::now().timestamp();
    let (header_payload, signature) = rsplit(token).ok_or(AuthError::InvalidFormat)?;
    let (_, payload) = rsplit(header_payload).ok_or(AuthError::InvalidFormat)?;

    let signature = base64::decode_config(signature, base64::URL_SAFE_NO_PAD)
        .map_err(|_| AuthError::InvalidFormat)?;
    let mut hmac = auth.hmac.clone();
    hmac.update(header_payload);
    hmac.verify_slice(&signature)
        .map_err(|_| AuthError::HmacFailed)?;

    let bytes = base64::decode_config(payload, base64::URL_SAFE_NO_PAD)
        .map_err(|_| AuthError::InvalidPayload)?;
    let payload =
        serde_json::from_slice::<Payload>(&bytes).map_err(|_| AuthError::InvalidPayload)?;
    if payload.exp <= now {
        return Err(AuthError::Expired);
    }
    let mut pubkey = [0u8; 32];
    five8::decode_32(payload.user_metadata.pub_key, &mut pubkey)
        .map_err(|_| AuthError::InvalidPayload)?;

    Ok(Jwt {
        token: token_str,
        user_id: payload.sub,
        pubkey,
    })
}

impl Identity for Jwt {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthV1>,
    ) -> Result<Self, AuthError> {
        let http_header = req
            .headers()
            .get(AUTHORIZATION)
            .ok_or_else(|| AuthError::NoHeader(AUTHORIZATION))?
            .as_bytes();
        let token = http_header
            .strip_prefix(b"Bearer ")
            .ok_or(AuthError::InvalidFormat)?;
        jwt_verify_inner(token, auth)
    }
}

#[derive(Getters)]
pub struct ApiKey {
    #[get = "pub"]
    key: String,
    #[get = "pub"]
    user_id: UserId,
    #[get = "pub"]
    pubkey: [u8; 32],
}

static X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");

async fn apikey_verify_inner(key: &str, auth: &AuthV1) -> Result<ApiKey, AuthError> {
    let conn = auth.pool.get_admin_conn().await?;
    let user = conn.get_user_from_apikey(key).await?;
    Ok(ApiKey {
        key: key.to_owned(),
        user_id: user.user_id,
        pubkey: user.pubkey,
    })
}

impl Identity for ApiKey {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthV1>,
    ) -> Result<Self, AuthError> {
        let key = req
            .headers()
            .get(&X_API_KEY)
            .ok_or_else(|| AuthError::NoHeader(X_API_KEY.clone()))?;
        let key = key.to_str().map_err(|_| AuthError::InvalidFormat)?;
        if !key.starts_with(apikey::KEY_PREFIX) {
            return Err(AuthError::InvalidFormat);
        }

        apikey_verify_inner(key, auth).await
    }
}

#[derive(Getters)]
pub struct Unverified {
    #[get = "pub"]
    pubkey: [u8; 32],
}

impl Identity for Unverified {
    async fn verify<'a>(
        req: &'a HttpRequest,
        _: &'a web::ThinData<AuthV1>,
    ) -> Result<Self, AuthError> {
        let key = req
            .headers()
            .get(&AUTHORIZATION)
            .ok_or_else(|| AuthError::NoHeader(AUTHORIZATION))?
            .to_str()
            .map_err(|_| AuthError::InvalidFormat)?;
        let key = key.strip_prefix("Bearer ").unwrap_or(key);
        let mut pubkey = [0u8; 32];
        five8::decode_32(key, &mut pubkey).map_err(|_| AuthError::InvalidFormat)?;
        Ok(Unverified { pubkey })
    }
}

#[derive(Getters, Clone)]
pub struct FlowRunToken {
    #[get = "pub"]
    id: FlowRunId,
}

fn flow_run_token_verify_inner(key: &str, auth: &AuthV1) -> Result<FlowRunToken, AuthError> {
    let key = key
        .strip_prefix(FLOW_RUN_TOKEN_PREFIX)
        .ok_or(AuthError::InvalidFormat)?;
    let mut bytes = [0u8; 48];
    let written = base64::decode_config_slice(key, base64::URL_SAFE_NO_PAD, &mut bytes)
        .map_err(|_| AuthError::InvalidPayload)?;
    if written != bytes.len() {
        return Err(AuthError::InvalidPayload);
    }
    let hash = auth.sig.hash(&bytes[..16]);
    if hash == blake3::Hash::from_bytes(bytes[16..].try_into().unwrap()) {
        Ok(FlowRunToken {
            id: FlowRunId::from_bytes(bytes[..16].try_into().unwrap()),
        })
    } else {
        Err(AuthError::InvalidPayload)
    }
}

impl Identity for FlowRunToken {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthV1>,
    ) -> Result<Self, AuthError> {
        let key = req
            .headers()
            .get(&AUTHORIZATION)
            .ok_or_else(|| AuthError::NoHeader(AUTHORIZATION))?
            .to_str()
            .map_err(|_| AuthError::InvalidFormat)?;
        let key = key.strip_prefix("Bearer ").unwrap_or(key);

        flow_run_token_verify_inner(key, auth)
    }
}

#[derive(Getters, Clone)]
pub struct AuthenticatedUser {
    #[get = "pub"]
    user_id: UserId,
    #[get = "pub"]
    pubkey: [u8; 32],
}

impl Identity for AuthenticatedUser {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthV1>,
    ) -> Result<Self, AuthError> {
        let result = Jwt::verify(req, auth).await.map(|x| Self {
            user_id: x.user_id,
            pubkey: x.pubkey,
        });
        early_return!(control_flow(result));

        ApiKey::verify(req, auth).await.map(|x| Self {
            user_id: x.user_id,
            pubkey: x.pubkey,
        })
    }
}

pub struct Auth<T>(T);

impl<T> Deref for Auth<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromRequest for Auth<T>
where
    T: Identity,
{
    type Error = AuthError;
    type Future = LocalBoxFuture<'static, Result<Self, AuthError>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let req = req.clone();
        async move {
            let auth = req
                .app_data::<web::ThinData<AuthV1>>()
                .ok_or_else(|| AuthError::NotConfigured)?;
            T::verify(&req, auth).await.map(Auth)
        }
        .boxed_local()
    }
}

#[derive(Clone)]
pub enum AuthEither<One, Two> {
    One(One),
    Two(Two),
}

impl<One: Identity, Two: Identity> FromRequest for AuthEither<One, Two> {
    type Error = AuthError;
    type Future = LocalBoxFuture<'static, Result<Self, AuthError>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let req = req.clone();
        async move {
            let auth = req
                .app_data::<web::ThinData<AuthV1>>()
                .ok_or_else(|| AuthError::NotConfigured)?;
            let result = One::verify(&req, auth).await.map(AuthEither::One);
            early_return!(control_flow(result));
            Two::verify(&req, auth).await.map(AuthEither::Two)
        }
        .boxed_local()
    }
}

impl AuthEither<AuthenticatedUser, FlowRunToken> {
    pub async fn can_access_flow_run(
        &self,
        id: FlowRunId,
        pool: &DbPool,
    ) -> Result<bool, db::Error> {
        match self {
            AuthEither::One(user) => {
                let user_id = user.user_id();
                let info = pool.get_admin_conn().await?.get_flow_run_info(id).await?;
                Ok(info.user_id == *user_id || info.shared_with.contains(user_id))
            }
            AuthEither::Two(run) => Ok(*run.id() == id),
        }
    }

    pub fn is_user(&self, id: &UserId) -> bool {
        matches!(self, AuthEither::One(user) if user.user_id == *id)
    }

    pub fn is_flow_run(&self, id: &FlowRunId) -> bool {
        matches!(self, AuthEither::Two(run) if run.id == *id)
    }

    pub fn user_id(&self) -> Option<UserId> {
        match self {
            AuthEither::One(user) => Some(user.user_id),
            _ => None,
        }
    }
}
