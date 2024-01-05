use crate::{
    api::prelude::auth::TokenType,
    auth::{ApiAuth, JWTPayload},
    db_worker::{
        flow_run_worker::{self, FlowRunWorker, SubscribeEvents},
        messages::{Finished, SubscribeError, SubscriptionID},
        user_worker::{SigReqEvent, SubscribeSigReq},
        DBWorker, FindActor, GetUserWorker,
    },
    middleware::auth::Unauthorized as AuthError,
    Config,
};
use actix::{fut::wrap_future, Actor, ActorContext, ActorFutureExt, AsyncContext};
use actix_web::{dev::HttpServiceFactory, guard, web, HttpRequest};
use actix_web_actors::ws::{self, CloseCode, WebsocketContext};
use chrono::{DateTime, Utc};
use db::pool::DbPool;
use flow::flow_run_events;
use flow_lib::{BoxError, FlowRunId};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory {
    let auth = web::Data::new(config.all_auth(db));
    web::resource("")
        .app_data(auth)
        .wrap(config.cors())
        .route(web::route().guard(guard::Get()).to(ws_handler))
}

async fn ws_handler(
    auth: web::Data<ApiAuth>,
    db_worker: web::Data<actix::Addr<DBWorker>>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<actix_web::HttpResponse, crate::error::Error> {
    let resp = ws::start(
        WsConn {
            state: State::Initial(Initial {
                msg_count: 0,
                queue: Vec::new(),
                auth_service: auth.into_inner(),
                db_worker: (**db_worker).clone(),
            }),
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

/// Actor holding a user's websocket connection
pub struct WsConn {
    state: State,
}

enum State {
    Initial(Initial),
    Authenticated(Authenticated),
}

struct Initial {
    msg_count: u64,
    queue: Vec<WithId<WsMessage>>,
    auth_service: Arc<ApiAuth>,
    db_worker: actix::Addr<DBWorker>,
}

struct Authenticated {
    msg_count: u64,
    token: TokenType,
    subscribing: HashMap<SubscriptionID, Subscription>,
    db_worker: actix::Addr<DBWorker>,
}

impl Actor for WsConn {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        tracing::info!("started websocket connection");
    }
}

impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                tracing::error!("WS error: {}, stopping connection", error);
                ctx.close(Some(CloseCode::Invalid.into()));
                ctx.stop();
                return;
            }
        };

        if let ws::Message::Text(text) = item {
            let id = self.next_id();
            match serde_json::from_str::<WsMessage>(&text) {
                Ok(msg) => match &mut self.state {
                    State::Initial(state) => state.handle(WithId { id, msg }, ctx),
                    State::Authenticated(state) => state.handle(WithId { id, msg }, ctx),
                },
                Err(error) => error_response(ctx, id, &error),
            };
        }
    }
}

impl Initial {
    fn handle(&mut self, msg: WithId<WsMessage>, ctx: &mut WebsocketContext<WsConn>) {
        let WithId { id, msg } = msg;
        match msg {
            WsMessage::Authenticate(m) => {
                let auth = self.auth_service.clone();
                let token = m.token;
                let fut = wrap_future::<_, WsConn>(auth.ws_authenticate(token));
                let fut = fut.map(move |res, act, ctx| match res {
                    Ok(token) => {
                        let new_state = if let State::Initial(state) = &act.state {
                            Authenticated {
                                msg_count: state.msg_count,
                                token: token.clone(),
                                subscribing: HashMap::new(),
                                db_worker: state.db_worker.clone(),
                            }
                        } else {
                            unreachable!()
                        };
                        let old_state =
                            std::mem::replace(&mut act.state, State::Authenticated(new_state));
                        let old_state = if let State::Initial(state) = old_state {
                            state
                        } else {
                            unreachable!()
                        };
                        if let State::Authenticated(state) = &mut act.state {
                            for msg in old_state.queue {
                                state.handle(msg, ctx);
                            }
                        } else {
                            unreachable!();
                        }
                        let user_id = token.user_id();
                        let flow_run_id = token.flow_run_id();
                        success_response(
                            ctx,
                            id,
                            json!({ "user_id": user_id, "flow_run_id": flow_run_id }),
                        )
                    }
                    Err(error) => error_response(ctx, id, &error),
                });
                ctx.wait(fut);
            }
            _ => self.queue.push(WithId { id, msg }),
        }
    }
}

impl Authenticated {
    fn handle(&mut self, msg: WithId<WsMessage>, ctx: &mut WebsocketContext<WsConn>) {
        let WithId { id, msg } = msg;
        match msg {
            WsMessage::Authenticate(_) => error_response(ctx, id, &"already authenticated"),
            WsMessage::SubscribeFlowRunEvents(msg) => self.subscribe_run(WithId { id, msg }, ctx),
            WsMessage::SubscribeSignatureRequests(msg) => {
                self.subscribe_sig(WithId { id, msg }, ctx)
            }
        }
    }

    fn subscribe_run(
        &mut self,
        msg: WithId<SubscribeFlowRunEvents>,
        ctx: &mut WebsocketContext<WsConn>,
    ) {
        let WithId { id, msg } = msg;
        let db_worker = self.db_worker.clone();
        let flow_run_id = msg.flow_run_id;
        if self.token.flow_run_id().is_some() && self.token.flow_run_id().unwrap() != flow_run_id {
            // TODO: implement token for interflow
            error_response(ctx, id, &"token did not match");
            return;
        }
        let addr = ctx.address();
        let token = self.token.clone();
        let fut = wrap_future::<_, WsConn>(async move {
            let (sub_id, events) = db_worker
                .send(FindActor::<FlowRunWorker>::new(msg.flow_run_id))
                .await?
                .ok_or("not found")?
                .send(SubscribeEvents {
                    user_id: token.user_id().unwrap_or_default(),
                    flow_run_id,
                    finished: addr.clone().into(),
                    receiver: addr.clone().into(),
                    receiver1: addr.into(),
                })
                .await??;

            Ok::<_, BoxError>((sub_id, events))
        })
        .map(move |res, act, ctx| match res {
            Ok((sub_id, events)) => {
                let state = if let State::Authenticated(state) = &mut act.state {
                    state
                } else {
                    unreachable!();
                };

                tracing::info!("subscribed flow-run");
                state.subscribing.insert(sub_id, Subscription {});
                success_response(ctx, id, json!({ "subscription_id": sub_id }));
                for event in events {
                    text_stream(
                        ctx,
                        sub_id,
                        &FlowRun {
                            flow_run_id,
                            time: event.time(),
                            content: event,
                        },
                    );
                }
            }
            Err(error) => error_response(ctx, id, &error),
        });
        ctx.spawn(fut);
    }

    fn subscribe_sig(
        &mut self,
        msg: WithId<SubscribeSignatureRequests>,
        ctx: &mut WebsocketContext<WsConn>,
    ) {
        let WithId { id, .. } = msg;

        let user_id = match self.token.user_id() {
            Some(user_id) => user_id,
            None => {
                error_response(ctx, id, &"cannot use when");
                return;
            }
        };

        let db_worker = self.db_worker.clone();
        let addr = ctx.address();
        let fut = wrap_future::<_, WsConn>(async move {
            let sub_id = db_worker
                .send(GetUserWorker { user_id })
                .await?
                .send(SubscribeSigReq {
                    user_id,
                    receiver: addr.into(),
                })
                .await??;

            Ok::<_, BoxError>(sub_id)
        })
        .map(move |res, act, ctx| match res {
            Ok(sub_id) => {
                let state = if let State::Authenticated(state) = &mut act.state {
                    state
                } else {
                    unreachable!();
                };

                tracing::info!("subscribed signature requests");
                state.subscribing.insert(sub_id, Subscription {});
                success_response(ctx, id, json!({ "subscription_id": sub_id }));
            }
            Err(error) => error_response(ctx, id, &error),
        });
        ctx.spawn(fut);
    }
}

impl WsConn {
    fn next_id(&mut self) -> u64 {
        let count = match &mut self.state {
            State::Initial(s) => &mut s.msg_count,
            State::Authenticated(s) => &mut s.msg_count,
        };
        let id = *count;
        *count += 1;
        id
    }
}

fn error_response<E: ToString>(ctx: &mut WebsocketContext<WsConn>, id: u64, error: &E) {
    let text = serde_json::to_string(&WSResponse::<()> {
        id,
        data: Err(error.to_string()),
    })
    .unwrap();
    ctx.text(text);
}

fn success_response<T: Serialize>(ctx: &mut WebsocketContext<WsConn>, id: u64, value: T) {
    let result = serde_json::to_string(&WSResponse::<T> {
        id,
        data: Ok(value),
    });
    match result {
        Ok(text) => ctx.text(text),
        Err(error) => error_response(
            ctx,
            id,
            &format!("InternalError: failed to serialize event, {}", error),
        ),
    }
}

fn text_stream<T: Serialize>(ctx: &mut WebsocketContext<WsConn>, sub_id: SubscriptionID, event: T) {
    let result = serde_json::to_string(&WSEvent::<T> { sub_id, event });
    match result {
        Ok(text) => ctx.text(text),
        Err(error) => tracing::error!("failed to serialize event: {}", error),
    }
}

#[derive(Serialize, Deserialize)]
pub enum WsMessage {
    Authenticate(WsAuthenticate),
    SubscribeFlowRunEvents(SubscribeFlowRunEvents),
    SubscribeSignatureRequests(SubscribeSignatureRequests),
}

#[derive(Serialize, Deserialize)]
pub struct WSResponse<T> {
    id: u64,
    #[serde(flatten)]
    data: Result<T, String>,
}

#[derive(Serialize, Deserialize)]
pub struct WSEvent<T> {
    sub_id: SubscriptionID,
    event: T,
}

struct WithId<M> {
    id: u64,
    msg: M,
}

impl<M: actix::Message> actix::Message for WithId<M> {
    type Result = WithId<M::Result>;
}

#[derive(Serialize, Deserialize)]
pub struct WsAuthenticate {
    token: String,
}

impl actix::Message for WsAuthenticate {
    type Result = Result<JWTPayload, AuthError>;
}

#[derive(Serialize, Deserialize)]
pub struct SubscribeFlowRunEvents {
    flow_run_id: FlowRunId,
}

impl actix::Message for SubscribeFlowRunEvents {
    type Result = Option<Result<SubscriptionID, SubscribeError>>;
}

#[derive(Serialize, Deserialize)]
pub struct SubscribeSignatureRequests {}

impl actix::Message for SubscribeSignatureRequests {
    type Result = Option<Result<SubscriptionID, SubscribeError>>;
}

impl actix::Handler<Finished> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: Finished, ctx: &mut Self::Context) -> Self::Result {
        if let State::Authenticated(state) = &mut self.state {
            if state.subscribing.remove(&msg.sub_id).is_some() {
                text_stream(ctx, msg.sub_id, "Done");
            }
        }
    }
}

impl actix::Handler<flow_run_worker::FullEvent> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: flow_run_worker::FullEvent, ctx: &mut Self::Context) -> Self::Result {
        text_stream(
            ctx,
            msg.sub_id,
            &FlowRun {
                flow_run_id: msg.flow_run_id,
                time: msg.event.time(),
                content: msg.event,
            },
        );
    }
}

#[derive(serde::Serialize)]
struct FlowRun {
    flow_run_id: FlowRunId,
    time: DateTime<Utc>,
    content: flow_run_events::Event,
}

impl actix::Handler<SigReqEvent> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: SigReqEvent, ctx: &mut Self::Context) -> Self::Result {
        let pubkey = bs58::encode(&msg.pubkey).into_string();
        let message = base64::encode(&msg.message);
        let req_id = msg.req_id;
        text_stream(
            ctx,
            msg.sub_id,
            &json!({
                "req_id": req_id,
                "pubkey": pubkey,
                "message": message,
            }),
        );
    }
}

struct Subscription {}
