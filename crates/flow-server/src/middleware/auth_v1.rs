use crate::{
    error::ErrorBody,
    user::{SignatureAuth, FLOW_RUN_TOKEN_PREFIX},
};
use actix_web::{
    http::{
        header::{HeaderName, AUTHORIZATION},
        StatusCode,
    },
    web::{self, ServiceConfig},
    FromRequest, HttpRequest, ResponseError,
};
use chrono::Utc;
use db::{
    apikey,
    pool::{DbPool, RealDbPool},
};
use flow_lib::{FlowRunId, UserId};
use futures_util::{future::LocalBoxFuture, FutureExt};
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

fn rsplit(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let dot = b.iter().rposition(|c| *c == b'.')?;
    Some((&b[..dot], &b[dot + 1..]))
}

trait Identity: Sized {
    fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthState>,
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

pub fn verify_jwt(mut hmac: Hmac<Sha256>, http_header: &[u8], now: i64) -> Result<Jwt, AuthError> {
    let token = http_header
        .strip_prefix(b"Bearer ")
        .ok_or(AuthError::InvalidFormat)?;
    let (header_payload, signature) = rsplit(token).ok_or(AuthError::InvalidFormat)?;
    let (_, payload) = rsplit(header_payload).ok_or(AuthError::InvalidFormat)?;

    let signature = base64::decode_config(signature, base64::URL_SAFE_NO_PAD)
        .map_err(|_| AuthError::InvalidFormat)?;
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

    Err(AuthError::NotConfigured)
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
    let Some(ref jwt_key) = server_config.supabase.jwt_key else {
        return;
    };
    let hmac = Hmac::new_from_slice(jwt_key.as_bytes()).unwrap();
    let DbPool::Real(pool) = db else { return };
    let sig = SignatureAuth::new(server_config.blake3_key);
    let state = AuthState {
        hmac,
        pool: pool.clone(),
        sig,
    };
    cfg.app_data(web::ThinData(state));
}

#[derive(Clone)]
struct AuthState {
    hmac: Hmac<Sha256>,
    pool: RealDbPool,
    sig: SignatureAuth,
}

pub struct Jwt {
    #[allow(dead_code)]
    token: String,
    user_id: UserId,
    pubkey: [u8; 32],
}

impl Identity for Jwt {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthState>,
    ) -> Result<Self, AuthError> {
        let http_header = req
            .headers()
            .get(AUTHORIZATION)
            .ok_or_else(|| AuthError::NoHeader(AUTHORIZATION))?
            .as_bytes();
        let now = Utc::now().timestamp();
        let token = http_header
            .strip_prefix(b"Bearer ")
            .ok_or(AuthError::InvalidFormat)?;
        let token_str =
            String::from_utf8(token.to_owned()).map_err(|_| AuthError::InvalidFormat)?;
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

        Ok(Self {
            token: token_str,
            user_id: payload.sub,
            pubkey,
        })
    }
}

pub struct ApiKey {
    #[allow(dead_code)]
    key: String,
    user_id: UserId,
    pubkey: [u8; 32],
}

static X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");

impl Identity for ApiKey {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthState>,
    ) -> Result<Self, AuthError> {
        let key = req
            .headers()
            .get(&X_API_KEY)
            .ok_or_else(|| AuthError::NoHeader(X_API_KEY.clone()))?;
        let key = key.to_str().map_err(|_| AuthError::InvalidFormat)?;
        if !key.starts_with(apikey::KEY_PREFIX) {
            return Err(AuthError::InvalidFormat);
        }

        let conn = auth.pool.get_admin_conn().await?;
        let user = conn.get_user_from_apikey(key).await?;
        Ok(ApiKey {
            key: key.to_owned(),
            user_id: user.user_id,
            pubkey: user.pubkey,
        })
    }
}

#[derive(Getters)]
pub struct Unverified {
    #[getset(get = "pub")]
    pubkey: [u8; 32],
}

impl Identity for Unverified {
    async fn verify<'a>(
        req: &'a HttpRequest,
        _: &'a web::ThinData<AuthState>,
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

pub struct FlowRunToken {
    #[allow(dead_code)]
    id: FlowRunId,
}

impl Identity for FlowRunToken {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthState>,
    ) -> Result<Self, AuthError> {
        let key = req
            .headers()
            .get(&AUTHORIZATION)
            .ok_or_else(|| AuthError::NoHeader(AUTHORIZATION))?
            .to_str()
            .map_err(|_| AuthError::InvalidFormat)?;
        let key = key.strip_prefix("Bearer ").unwrap_or(key);
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
}

#[derive(Getters)]
pub struct AuthenticatedUser {
    #[getset(get = "pub")]
    user_id: UserId,
    #[getset(get = "pub")]
    pubkey: [u8; 32],
}

impl Identity for AuthenticatedUser {
    async fn verify<'a>(
        req: &'a HttpRequest,
        auth: &'a web::ThinData<AuthState>,
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
                .app_data::<web::ThinData<AuthState>>()
                .ok_or_else(|| AuthError::NotConfigured)?;
            T::verify(&req, auth).await.map(Auth)
        }
        .boxed_local()
    }
}

pub enum Auth2<One, Two> {
    One(One),
    Two(Two),
}

fn control_flow<T>(r: Result<T, AuthError>) -> ControlFlow<Result<T, AuthError>> {
    if matches!(&r, Err(e) if e.try_again()) {
        ControlFlow::Continue(())
    } else {
        ControlFlow::Break(r)
    }
}

impl<One: Identity, Two: Identity> FromRequest for Auth2<One, Two> {
    type Error = AuthError;
    type Future = LocalBoxFuture<'static, Result<Self, AuthError>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let req = req.clone();
        async move {
            let auth = req
                .app_data::<web::ThinData<AuthState>>()
                .ok_or_else(|| AuthError::NotConfigured)?;
            let result = One::verify(&req, auth).await.map(Auth2::One);
            early_return!(control_flow(result));
            Two::verify(&req, auth).await.map(Auth2::Two)
        }
        .boxed_local()
    }
}
