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
use actix::{fut::wrap_future, Actor, ActorContext, ActorFutureExt, AsyncContext, WrapFuture};
use actix_web::{dev::HttpServiceFactory, guard, web, HttpRequest};
use actix_web_actors::ws::{self, CloseCode, WebsocketContext};
use chrono::{DateTime, Utc};
use db::pool::DbPool;
use flow::flow_run_events;
use flow_lib::{BoxError, FlowRunId};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
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
            msg_count: 0,
            tokens: <_>::default(),
            queue: <_>::default(),
            subscribing: <_>::default(),

            auth_service: auth.into_inner(),
            db_worker: (**db_worker).clone(),
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

/// Actor holding a user's websocket connection
pub struct WsConn {
    msg_count: u64,
    tokens: HashSet<TokenType>,
    queue: Vec<WithId<WsMessage>>,
    subscribing: HashMap<SubscriptionID, Subscription>,

    auth_service: Arc<ApiAuth>,
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
        match item {
            Ok(ws::Message::Text(text)) => {
                let id = self.next_id();
                match serde_json::from_str::<rpc::Request<WsMessage>>(&text) {
                    Ok(msg) => match msg.request {
                        WsMessage::Authenticate(params) => self.authenticate(msg.id, params, ctx),
                        WsMessage::SubscribeFlowRunEvents(params) => {
                            self.subscribe_run(msg.id, params, ctx)
                        }
                        WsMessage::SubscribeSignatureRequests(params) => {
                            self.subscribe_sig(msg.id, params, ctx)
                        }
                    },
                    Err(error) => error_response(ctx, id, &error),
                };
            }
            Ok(ws::Message::Ping(data)) => {
                ctx.pong(&data);
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(error) => {
                tracing::error!("WS error: {}, stopping connection", error);
                ctx.close(Some(CloseCode::Invalid.into()));
                ctx.stop();
            }
            Ok(ws::Message::Binary(_)) => {
                tracing::warn!("received Binary message");
            }
            Ok(ws::Message::Continuation(_)) => {
                tracing::warn!("received Continuation message");
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Nop) => {}
        }
    }
}

impl WsConn {
    fn handle_decoded_message(
        &mut self,
        msg: WithId<WsMessage>,
        ctx: &mut WebsocketContext<WsConn>,
    ) {
        let WithId { id, msg } = msg;
        match msg {
            WsMessage::Authenticate(msg) => self.authenticate(WithId { id, msg }, ctx),
            WsMessage::SubscribeFlowRunEvents(msg) => self.subscribe_run(WithId { id, msg }, ctx),
            WsMessage::SubscribeSignatureRequests(msg) => {
                self.subscribe_sig(WithId { id, msg }, ctx)
            }
        }
    }

    fn authenticate(
        &mut self,
        id: i64,
        params: WsAuthenticate,
        ctx: &mut WebsocketContext<WsConn>,
    ) {
        let token = params.token;
        let fut = self
            .auth_service
            .clone()
            .ws_authenticate(token)
            .into_actor(&*self)
            .map(move |res, act, ctx| match res {
                Ok(token) => {
                    act.tokens.insert(token);
                    for msg in act.queue.split_off(0) {
                        act.handle_decoded_message(msg, ctx);
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

    fn subscribe_run(
        &mut self,
        id: i64,
        params: SubscribeFlowRunEvents,
        ctx: &mut WebsocketContext<WsConn>,
    ) {
        // TODO: implement token for interflow
        let flow_run_id = params.flow_run_id;
        let db_worker = self.db_worker.clone();
        let tokens = self.tokens.clone();
        let addr = ctx.address();
        let fut = async move {
            Ok::<_, BoxError>(
                db_worker
                    .send(FindActor::<FlowRunWorker>::new(flow_run_id))
                    .await?
                    .ok_or("not found")?
                    .send(SubscribeEvents {
                        tokens,
                        finished: addr.clone().into(),
                        receiver: addr.clone().into(),
                    })
                    .await??,
            )
        }
        .into_actor(&*self)
        .map(move |res, act, ctx| match res {
            Ok((stream_id, events)) => {
                tracing::info!("subscribed flow-run");
                act.subscribing.insert(stream_id, Subscription {});
                success_response(ctx, id, json!({ "subscription": stream_id }));
                for event in events {
                    text_stream(
                        ctx,
                        stream_id,
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
        id: i64,
        _params: SubscribeSignatureRequests,
        ctx: &mut WebsocketContext<WsConn>,
    ) {
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
            let stream_id = db_worker
                .send(GetUserWorker { user_id })
                .await?
                .send(SubscribeSigReq {
                    user_id,
                    receiver: addr.into(),
                })
                .await??;

            Ok::<_, BoxError>(stream_id)
        })
        .map(move |res, act, ctx| match res {
            Ok(stream_id) => {
                let state = if let State::Authenticated(state) = &mut act.state {
                    state
                } else {
                    unreachable!();
                };

                tracing::info!("subscribed signature requests");
                state.subscribing.insert(stream_id, Subscription {});
                success_response(ctx, id, json!({ "subscription_id": stream_id }));
            }
            Err(error) => error_response(ctx, id, &error),
        });
        ctx.spawn(fut);
    }
}

impl Authenticated {}

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

fn error_response<E: ToString>(ctx: &mut WebsocketContext<WsConn>, id: i64, error: &E) {
    let text = serde_json::to_string(&WSResponse::<()> {
        id,
        data: Err(error.to_string()),
    })
    .unwrap();
    ctx.text(text);
}

fn success_response<T: Serialize>(ctx: &mut WebsocketContext<WsConn>, id: i64, value: T) {
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

fn text_stream<T: Serialize>(ctx: &mut WebsocketContext<WsConn>, stream_id: SubscriptionID, event: T) {
    let result = serde_json::to_string(&WSEvent::<T> { stream_id, event });
    match result {
        Ok(text) => ctx.text(text),
        Err(error) => tracing::error!("failed to serialize event: {}", error),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum WsMessage {
    Authenticate(WsAuthenticate),
    SubscribeFlowRunEvents(SubscribeFlowRunEvents),
    SubscribeSignatureRequests(SubscribeSignatureRequests),
}

#[derive(Serialize, Deserialize)]
pub struct WSResponse<T> {
    id: i64,
    #[serde(flatten)]
    data: Result<T, String>,
}

#[derive(Serialize, Deserialize)]
pub struct WSEvent<T> {
    stream_id: SubscriptionID,
    event: T,
}

struct WithId<M> {
    id: i64,
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
            if state.subscribing.remove(&msg.stream_id).is_some() {
                text_stream(ctx, msg.stream_id, "Done");
            }
        }
    }
}

impl actix::Handler<flow_run_worker::FullEvent> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: flow_run_worker::FullEvent, ctx: &mut Self::Context) -> Self::Result {
        text_stream(
            ctx,
            msg.stream_id,
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
        let id = msg.id;
        text_stream(
            ctx,
            msg.stream_id,
            &json!({
                "id": id,
                "pubkey": pubkey,
                "message": message,
            }),
        );
    }
}

struct Subscription {}
