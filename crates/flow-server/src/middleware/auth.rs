use crate::{
    api::{auth_proxy, ws_auth_proxy},
    error::ErrorBody,
};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{
        header::{
            from_one_raw_str, Header, HeaderName, HeaderValue, InvalidHeaderValue,
            TryIntoHeaderValue, AUTHORIZATION,
        },
        StatusCode,
    },
    HttpMessage, HttpResponse, ResponseError,
};
use db::{
    apikey,
    pool::{ProxiedDbPool, RealDbPool},
};
use flow_lib::UserId;
use futures_util::{future::LocalBoxFuture, FutureExt};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{convert::Infallible, future::Ready, panic::Location, rc::Rc, str::FromStr, sync::Arc};
use thiserror::Error as ThisError;
use utils::bs58_decode;

pub const X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");

pub struct ApiKey(pub String);

impl ApiKey {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for ApiKey {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl TryIntoHeaderValue for ApiKey {
    type Error = InvalidHeaderValue;

    fn try_into_value(self) -> Result<HeaderValue, Self::Error> {
        HeaderValue::from_str(&self.0)
    }
}

impl Header for ApiKey {
    fn name() -> HeaderName {
        X_API_KEY
    }

    fn parse<M: HttpMessage>(msg: &M) -> Result<Self, actix_web::error::ParseError> {
        from_one_raw_str(msg.headers().get(Self::name()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub api_key: Option<String>,
    pub jwt: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct JWTPayload {
    pub user_id: UserId,
    #[serde(with = "utils::serde_bs58")]
    pub pubkey: [u8; 32],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Unverified {
    #[serde(with = "utils::serde_bs58")]
    pub pubkey: [u8; 32],
}

#[derive(ThisError, Debug)]
#[error("failed to verify access token, at {0}")]
pub struct Unauthorized(&'static Location<'static>);

#[track_caller]
fn unauthorized() -> Unauthorized {
    Unauthorized(Location::caller())
}

impl ResponseError for Unauthorized {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }

    fn error_response(&self) -> HttpResponse {
        ErrorBody::build(self)
    }
}

fn rsplit(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let dot = b.iter().rposition(|c| *c == b'.')?;
    Some((&b[..dot], &b[dot + 1..]))
}

fn split(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let dot = b.iter().position(|c| *c == b'.')?;
    Some((&b[..dot], &b[dot + 1..]))
}

fn jwt_verify(mut hmac: Hmac<Sha256>, token: &[u8], now: i64) -> Result<JWTPayload, Unauthorized> {
    let (header_payload, signature) = rsplit(token).ok_or_else(unauthorized)?;

    let signature =
        base64::decode_config(signature, base64::URL_SAFE).map_err(|_| unauthorized())?;
    hmac.update(header_payload);
    hmac.verify_slice(&signature).map_err(|_| unauthorized())?;
    let (_, payload) = split(header_payload).ok_or_else(unauthorized)?;
    let payload = decode_payload(payload, now)?;
    Ok(payload)
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

fn decode_payload(payload: &[u8], now: i64) -> Result<JWTPayload, Unauthorized> {
    let s = base64::decode_config(payload, base64::URL_SAFE).map_err(|_| unauthorized())?;
    let p = serde_json::from_slice::<Payload>(&s).map_err(|_| unauthorized())?;

    if p.exp <= now {
        return Err(unauthorized());
    }

    let mut pubkey = [0u8; 32];
    let size = bs58::decode(p.user_metadata.pub_key)
        .into(&mut pubkey)
        .map_err(|_| unauthorized())?;
    if size != 32 {
        return Err(unauthorized());
    }

    Ok(JWTPayload {
        user_id: p.sub,
        pubkey,
    })
}

#[derive(Clone)]
pub enum ApiAuth {
    Real(RealApiAuth),
    Proxied(ProxiedApiAuth),
}

#[derive(Clone)]
pub struct RealApiAuth {
    hmac: Hmac<Sha256>,
    anon_key: String,
    pool: RealDbPool,
}

fn decode_base58_pubkey(v: &HeaderValue) -> Result<[u8; 32], Unauthorized> {
    let s = std::str::from_utf8(v.as_bytes()).map_err(|_| unauthorized())?;
    bs58_decode::<32>(s).map_err(|_| unauthorized())
}

impl RealApiAuth {
    async fn run(&self, r: &ServiceRequest) -> Result<(), Unauthorized> {
        match r.headers().get("x-api-key") {
            Some(apikey) => {
                if apikey.as_bytes() == self.anon_key.as_bytes() {
                    self.jwt_verify_request(r, chrono::Utc::now().timestamp())
                } else {
                    self.apikey_auth(apikey, r).await
                }
            }
            None => self.jwt_verify_request(r, chrono::Utc::now().timestamp()),
        }
    }

    fn jwt_verify_request(&self, r: &ServiceRequest, now: i64) -> Result<(), Unauthorized> {
        let header = r.headers().get(&AUTHORIZATION).ok_or_else(unauthorized)?;
        let token = header.as_bytes().strip_prefix(b"Bearer ");

        match token {
            Some(token) => {
                let payload = jwt_verify(self.hmac.clone(), token, now)?;

                let mut ext = r.extensions_mut();
                ext.insert(payload);
                ext.insert(Token {
                    jwt: Some(String::from_utf8(token.to_owned()).map_err(|_| unauthorized())?),
                    api_key: None,
                });
                Ok(())
            }
            None => {
                let pubkey = decode_base58_pubkey(header)?;
                let mut ext = r.extensions_mut();
                ext.insert(Unverified { pubkey });
                Ok(())
            }
        }
    }

    async fn apikey_auth(
        &self,
        apikey: &HeaderValue,
        r: &ServiceRequest,
    ) -> Result<(), Unauthorized> {
        let apikey = apikey.to_str().map_err(|_| unauthorized())?;
        if !apikey.starts_with(apikey::KEY_PREFIX) {
            return Err(unauthorized());
        }
        let conn = self
            .pool
            .get_admin_conn()
            .await
            .map_err(|_| unauthorized())?;
        let user = conn
            .get_user_from_apikey(apikey)
            .await
            .map_err(|_| unauthorized())?;
        let mut ext = r.extensions_mut();
        ext.insert(JWTPayload {
            pubkey: user.pubkey,
            user_id: user.user_id,
        });
        ext.insert(Token {
            jwt: None,
            api_key: Some(apikey.to_owned()),
        });
        Ok(())
    }

    pub async fn ws_authenticate(&self, token: String) -> Result<JWTPayload, Unauthorized> {
        if token.starts_with(apikey::KEY_PREFIX) {
            let conn = self
                .pool
                .get_admin_conn()
                .await
                .map_err(|_| unauthorized())?;
            let user = conn
                .get_user_from_apikey(&token)
                .await
                .map_err(|_| unauthorized())?;
            Ok(JWTPayload {
                pubkey: user.pubkey,
                user_id: user.user_id,
            })
        } else {
            jwt_verify(
                self.hmac.clone(),
                token.as_bytes(),
                chrono::Utc::now().timestamp(),
            )
        }
    }
}

#[derive(Clone)]
pub struct ProxiedApiAuth {
    client: reqwest::Client,
    upstream_url: String,
}

impl ProxiedApiAuth {
    async fn run(&self, r: &ServiceRequest) -> Result<(), Unauthorized> {
        let mut req = self
            .client
            .post(format!("{}/proxy/auth", self.upstream_url));
        if let Some(value) = r.headers().get(AUTHORIZATION) {
            if let Ok(pubkey) = decode_base58_pubkey(value) {
                let mut ext = r.extensions_mut();
                ext.insert(Unverified { pubkey });
                return Ok(());
            }
            req = req.header(AUTHORIZATION, value);
        }
        if let Some(value) = r.headers().get(X_API_KEY) {
            req = req.header(X_API_KEY, value);
        }
        let output = req
            .send()
            .await
            .map_err(|_| unauthorized())?
            .json::<auth_proxy::Output>()
            .await
            .map_err(|_| unauthorized())?;
        let mut ext = r.extensions_mut();
        ext.insert(output.payload);
        ext.insert(output.token);
        Ok(())
    }

    pub async fn ws_authenticate(&self, token: String) -> Result<JWTPayload, Unauthorized> {
        Ok(self
            .client
            .post(format!("{}/proxy/ws_auth", self.upstream_url))
            .json(&ws_auth_proxy::Params { token })
            .send()
            .await
            .map_err(|_| unauthorized())?
            .json::<ws_auth_proxy::Output>()
            .await
            .map_err(|_| unauthorized())?
            .payload)
    }
}

impl ApiAuth {
    pub fn real(secret: &[u8], anon_key: String, pool: RealDbPool) -> Self {
        let hmac = Hmac::new_from_slice(secret).unwrap();
        ApiAuth::Real(RealApiAuth {
            hmac,
            anon_key,
            pool,
        })
    }

    pub fn proxied(pool: ProxiedDbPool) -> Self {
        ApiAuth::Proxied(ProxiedApiAuth {
            client: pool.client,
            upstream_url: pool.config.upstream_url,
        })
    }

    async fn run(&self, r: &ServiceRequest) -> Result<(), Unauthorized> {
        match self {
            ApiAuth::Real(x) => x.run(r).await,
            ApiAuth::Proxied(x) => x.run(r).await,
        }
    }

    pub async fn ws_authenticate(
        self: Arc<Self>,
        token: String,
    ) -> Result<JWTPayload, Unauthorized> {
        match self.as_ref() {
            ApiAuth::Real(x) => x.ws_authenticate(token).await,
            ApiAuth::Proxied(x) => x.ws_authenticate(token).await,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Transform = ApiAuthMiddleware<S>;
    type Response = <Self::Transform as Service<ServiceRequest>>::Response;
    type Error = <Self::Transform as Service<ServiceRequest>>::Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();

    fn new_transform(&self, s: S) -> Self::Future {
        std::future::ready(Ok(ApiAuthMiddleware {
            service: Rc::new(s),
            state: self.clone(),
        }))
    }
}

pub struct ApiAuthMiddleware<S> {
    service: Rc<S>,
    state: ApiAuth,
}

impl<S, B> Service<ServiceRequest> for ApiAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, r: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let state = self.state.clone();
        async move {
            match state.run(&r).await {
                Ok(_) => service
                    .call(r)
                    .await
                    .map(ServiceResponse::<B>::map_into_left_body),
                Err(e) => Ok(r.error_response(e).map_into_right_body()),
            }
        }
        .boxed_local()
    }
}
