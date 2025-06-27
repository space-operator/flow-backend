use crate::{
    api::{apikey_info, claim_token},
    auth::X_API_KEY,
    user::PasswordLogin,
};
use actix::{Actor, ActorFutureExt, Addr, AsyncContext, ResponseFuture, WrapFuture};
use chrono::{Duration, Utc};
use db::{LocalStorage, local_storage::Jwt, pool::RealDbPool};
use flow_lib::{UserId, config::Endpoints, context::get_jwt, utils::tower_client::CommonErrorExt};
use futures_channel::oneshot;
use futures_util::{FutureExt, future::BoxFuture};
use hashbrown::HashMap;
use reqwest::{StatusCode, header::HeaderName};
use serde::{Deserialize, Serialize};
use std::future::ready;
use utils::{actix_service::ActixService, address_book::ManagableActor};

pub trait ClaimToken: Unpin + 'static {
    fn claim(&self) -> BoxFuture<'static, Result<Jwt, get_jwt::Error>>;
}

#[derive(Clone)]
pub struct LoginWithAdminCred {
    pub client: reqwest::Client,
    pub user_id: UserId,
    pub db: RealDbPool,
    pub endpoints: Endpoints,
}

impl LoginWithAdminCred {
    pub async fn claim(self) -> Result<Jwt, get_jwt::Error> {
        let r = self
            .db
            .get_admin_conn()
            .await
            .map_err(get_jwt::Error::other)?
            .get_login_credential(self.user_id)
            .await
            .map_err(get_jwt::Error::other)?;

        let resp = self
            .client
            .post(format!(
                "{}/auth/v1/token?grant_type=password",
                self.endpoints.supabase
            ))
            .header(
                HeaderName::from_static("apikey"),
                &self.endpoints.supabase_anon_key,
            )
            .json(&PasswordLogin {
                email: r.email,
                password: r.password,
            })
            .send()
            .await
            .map_err(get_jwt::Error::other)?;

        if resp.status() != StatusCode::OK {
            return Err(supabase_error(resp).await);
        }

        Ok(Jwt::from(
            resp.json::<TokenResponse>()
                .await
                .map_err(get_jwt::Error::other)?,
        ))
    }
}

impl ClaimToken for LoginWithAdminCred {
    fn claim(&self) -> BoxFuture<'static, Result<Jwt, get_jwt::Error>> {
        Box::pin(self.clone().claim())
    }
}

#[derive(Clone)]
pub struct ClaimWithApiKey {
    pub client: reqwest::Client,
    pub user_id: UserId,
    pub api_key: String,
    pub upstream_url: String,
}

impl ClaimWithApiKey {
    async fn claim(self) -> Result<Jwt, get_jwt::Error> {
        let claim_token::Output {
            access_token,
            refresh_token,
            expires_at,
            ..
        } = self
            .client
            .post(format!("{}/auth/claim_token", self.upstream_url))
            .header(X_API_KEY.as_str(), self.api_key)
            .send()
            .await
            .map_err(get_jwt::Error::other)?
            .error_for_status()
            .map_err(get_jwt::Error::other)?
            .json::<claim_token::Output>()
            .await
            .map_err(get_jwt::Error::other)?;
        Ok(Jwt {
            access_token,
            refresh_token,
            expires_at,
        })
    }
}

impl ClaimToken for ClaimWithApiKey {
    fn claim(&self) -> BoxFuture<'static, Result<Jwt, get_jwt::Error>> {
        Box::pin(self.clone().claim())
    }
}

pub struct TokenWorker {
    claim: Box<dyn ClaimToken>,
    user_id: UserId,
    local_db: LocalStorage,
    state: TokenState,
    endpoints: Endpoints,
}

impl ManagableActor for TokenWorker {
    type ID = UserId;
    fn id(&self) -> Self::ID {
        self.user_id
    }
}

impl TokenWorker {
    pub fn new<T: ClaimToken>(
        user_id: UserId,
        local_db: LocalStorage,
        endpoints: Endpoints,
        claim: T,
    ) -> Self {
        Self {
            claim: Box::new(claim),
            user_id,
            local_db,
            endpoints,
            state: TokenState::None,
        }
    }
}

enum TokenState {
    None,
    Available(Jwt),
    Fetching(Vec<oneshot::Sender<<get_jwt::Request as actix::Message>::Result>>),
}

impl Actor for TokenWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        tracing::debug!("started TokenWorker {}", self.user_id);
        let token = self
            .local_db
            .get_jwt(&self.user_id)
            .map_err(|e| {
                tracing::error!("{}", e);
                self.local_db
                    .remove_jwt(&self.user_id)
                    .map_err(|e| tracing::error!("{}", e))
                    .ok();
            })
            .ok()
            .flatten()
            .map(TokenState::Available)
            .unwrap_or(TokenState::None);
        self.state = token;
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::debug!("stopped TokenWorker {}", self.user_id);
    }
}

#[derive(Deserialize, Debug)]
struct GoTrueError {
    error: String,
    error_description: String,
}

async fn supabase_error(resp: reqwest::Response) -> get_jwt::Error {
    let bytes = match resp.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => return get_jwt::Error::other(error),
    };
    match serde_json::from_slice::<GoTrueError>(&bytes) {
        Ok(GoTrueError {
            error,
            error_description,
        }) => get_jwt::Error::Supabase {
            error,
            error_description,
        },
        Err(_) => {
            let msg = String::from_utf8_lossy(&bytes).into_owned();
            get_jwt::Error::msg(msg)
        }
    }
}

impl From<TokenResponse> for Jwt {
    fn from(resp: TokenResponse) -> Self {
        Self {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
            expires_at: Utc::now() + Duration::try_seconds(resp.expires_in as i64).unwrap(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    // token_type: String,
    expires_in: u32,
    refresh_token: String,
}

async fn refresh(refresh_token: String, endpoints: Endpoints) -> Result<Jwt, get_jwt::Error> {
    #[derive(Serialize, Debug)]
    struct RefreshToken {
        refresh_token: String,
    }

    let resp = reqwest::Client::new()
        .post(format!(
            "{}/auth/v1/token?grant_type=refresh_token",
            endpoints.supabase
        ))
        .header(
            HeaderName::from_static("apikey"),
            &endpoints.supabase_anon_key,
        )
        .json(&RefreshToken { refresh_token })
        .send()
        .await
        .map_err(get_jwt::Error::other)?;
    if resp.status() != StatusCode::OK {
        return Err(supabase_error(resp).await);
    }
    Ok(Jwt::from(
        resp.json::<TokenResponse>()
            .await
            .map_err(get_jwt::Error::other)?,
    ))
}

impl TokenState {
    fn process_result(
        &mut self,
        res: Result<Jwt, get_jwt::Error>,
        local: &LocalStorage,
        user_id: &UserId,
    ) {
        *self = match std::mem::replace(self, TokenState::None) {
            TokenState::None => unreachable!(),
            TokenState::Available(_) => unreachable!(),
            TokenState::Fetching(vec) => {
                let (result, state) = match res {
                    Ok(jwt) => {
                        local
                            .set_jwt(user_id, &jwt)
                            .map_err(|e| tracing::error!("{}", e))
                            .ok();
                        (
                            Ok(get_jwt::Response {
                                access_token: jwt.access_token.clone(),
                            }),
                            TokenState::Available(jwt),
                        )
                    }
                    Err(error) => {
                        local
                            .remove_jwt(user_id)
                            .map_err(|e| tracing::error!("{}", e))
                            .ok();
                        (Err(error), TokenState::None)
                    }
                };
                for tx in vec {
                    tx.send(result.clone()).ok();
                }
                state
            }
        }
    }
}

impl actix::Handler<get_jwt::Request> for TokenWorker {
    type Result = ResponseFuture<<get_jwt::Request as actix::Message>::Result>;
    fn handle(&mut self, msg: get_jwt::Request, ctx: &mut Self::Context) -> Self::Result {
        if self.user_id != msg.user_id {
            return Box::pin(ready(Err(get_jwt::Error::WrongRecipient)));
        }

        let result: Self::Result;
        let from_rx = |rx: oneshot::Receiver<_>| {
            Box::pin(async move { rx.await.map_err(|_| get_jwt::Error::msg("canceled"))? })
        };
        self.state =
            match std::mem::replace(&mut self.state, TokenState::None) {
                TokenState::None => {
                    tracing::info!("claim new JWT token, user_id={}", self.user_id);
                    let task = self.claim.claim().into_actor(&*self).map(|res, act, _| {
                        act.state.process_result(res, &act.local_db, &act.user_id)
                    });
                    ctx.spawn(task);

                    let (tx, rx) = oneshot::channel();
                    result = from_rx(rx);
                    TokenState::Fetching([tx].into())
                }
                TokenState::Available(jwt) => {
                    if jwt.expires_at - Utc::now() < Duration::try_minutes(5).unwrap() {
                        let refresh_token = jwt.refresh_token;
                        let endpoints = self.endpoints.clone();
                        tracing::info!("refresh JWT token, user_id={}", self.user_id);
                        let task = refresh(refresh_token, endpoints).into_actor(&*self).map(
                            |res, act, _| {
                                act.state.process_result(res, &act.local_db, &act.user_id)
                            },
                        );
                        ctx.spawn(task);

                        let (tx, rx) = oneshot::channel();
                        result = from_rx(rx);
                        TokenState::Fetching(vec![tx])
                    } else {
                        result = Box::pin(ready(Ok(get_jwt::Response {
                            access_token: jwt.access_token.clone(),
                        })));
                        TokenState::Available(jwt)
                    }
                }
                TokenState::Fetching(mut vec) => {
                    let (tx, rx) = oneshot::channel();
                    vec.push(tx);
                    result = from_rx(rx);
                    TokenState::Fetching(vec)
                }
            };

        result
    }
}

pub async fn token_from_apikeys(
    keys: Vec<String>,
    local_db: LocalStorage,
    endpoints: Endpoints,
    upstream_url: String,
) -> (
    HashMap<UserId, get_jwt::Svc>,
    HashMap<UserId, actix::Addr<TokenWorker>>,
) {
    let client = reqwest::Client::new();
    let tasks = keys.into_iter().map(|k| {
        async fn send(
            client: &reqwest::Client,
            url: &str,
            k: String,
        ) -> Result<UserId, reqwest::Error> {
            let resp = client
                .get(format!("{url}/apikey/info"))
                .header(X_API_KEY.as_str(), k)
                .send()
                .await?;
            let output = resp
                .error_for_status()?
                .json::<apikey_info::Output>()
                .await?;
            Ok(output.user_id)
        }

        send(&client, &upstream_url, k.clone()).map(|r| (k, r))
    });
    let actors = futures_util::future::join_all(tasks)
        .await
        .into_iter()
        .filter_map(|(k, r)| match r {
            Ok(id) => Some((id, k)),
            Err(error) => {
                tracing::error!("failed to get key info {:?}: {}", k, error);
                None
            }
        })
        .map(|(user_id, api_key)| {
            let worker = TokenWorker::new(
                user_id,
                local_db.clone(),
                endpoints.clone(),
                ClaimWithApiKey {
                    client: client.clone(),
                    user_id,
                    api_key,
                    upstream_url: upstream_url.clone(),
                },
            );
            let addr = worker.start();
            (user_id, addr)
        })
        .collect::<HashMap<UserId, Addr<TokenWorker>>>();

    let services = actors
        .iter()
        .map(|(user_id, addr)| {
            let svc = get_jwt::Svc::new(ActixService::from(addr.clone().recipient()));
            (*user_id, svc)
        })
        .collect::<HashMap<UserId, get_jwt::Svc>>();
    (services, actors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn need_key_refresh() {
        let error = refresh(
            "Hello".to_owned(),
            Endpoints {
                flow_server: String::new(),
                supabase: "https://base.spaceoperator.com".to_owned(),
                supabase_anon_key: std::env::var("ANON_KEY").unwrap(),
            },
        )
        .await
        .unwrap_err();
        dbg!(error);
    }
}
