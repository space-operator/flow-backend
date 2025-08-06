use crate::{
    Config,
    api::prelude::auth::TokenType,
    auth::ApiAuth,
    db_worker::{
        DBWorker, FindActor, GetUserWorker,
        flow_run_worker::{FlowRunWorker, SubscribeEvents},
        messages::SubscriptionID,
        user_worker::SubscribeSigReq,
    },
};
use actix::{
    Actor, ActorContext, ActorFutureExt, ActorStreamExt, AsyncContext, SystemService, WrapFuture,
    WrapStream,
};
use actix_web::{HttpRequest, dev::HttpServiceFactory, guard, web};
use actix_web_actors::ws::{self, CloseCode, WebsocketContext};
use db::pool::DbPool;
use flow_lib::{BoxError, FlowRunId, flow_run_events::Event};
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
struct Request {
    id: i64,
    #[serde(flatten)]
    data: WsMessage,
}

#[derive(Deserialize)]
#[serde(tag = "method", content = "params")]
enum WsMessage {
    Authenticate(Authenticate),
    SubscribeFlowRunEvents(SubscribeFlowRunEvents),
    SubscribeSignatureRequests(SubscribeSignatureRequests),
}

#[derive(Deserialize)]
struct Authenticate {
    token: String,
}

#[derive(Deserialize)]
pub struct SubscribeFlowRunEvents {
    flow_run_id: FlowRunId,
    #[serde(default)]
    token: Option<String>,
}

#[derive(Deserialize)]
pub struct SubscribeSignatureRequests {}

#[derive(Serialize, Deserialize)]
pub struct WsResponse<T> {
    id: i64,
    #[serde(flatten)]
    data: Result<T, String>,
}

#[derive(Serialize, Deserialize)]
pub struct WsEvent<T> {
    stream_id: SubscriptionID,
    #[serde(flatten)]
    event: T,
}

pub fn service(config: &Config, db: DbPool) -> impl HttpServiceFactory + 'static {
    let auth = web::Data::new(config.all_auth(db));
    web::resource("")
        .app_data(auth)
        .wrap(config.cors())
        .route(web::route().guard(guard::Get()).to(ws_handler))
}

async fn ws_handler(
    auth: web::Data<ApiAuth>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<actix_web::HttpResponse, crate::error::Error> {
    let resp = ws::start(
        WsConn {
            tokens: <_>::default(),
            subscribing: <_>::default(),

            auth_service: auth.into_inner(),
            db_worker: DBWorker::from_registry(),
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

/// Actor holding a user's websocket connection
pub struct WsConn {
    tokens: HashSet<TokenType>,
    subscribing: HashSet<SubscriptionID>,

    auth_service: Arc<ApiAuth>,
    db_worker: actix::Addr<DBWorker>,
}

impl Actor for WsConn {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        tracing::info!("started websocket connection");
    }
}

fn find_id(msg: &str) -> Option<i64> {
    #[derive(Deserialize)]
    struct Id {
        id: i64,
    }
    Some(serde_json::from_str::<Id>(msg).ok()?.id)
}

impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => {
                tracing::debug!("received text '{}'", text);
                match serde_json::from_str::<Request>(&text) {
                    Ok(msg) => match msg.data {
                        WsMessage::Authenticate(params) => self.authenticate(msg.id, params, ctx),
                        WsMessage::SubscribeFlowRunEvents(params) => {
                            self.subscribe_run(msg.id, params, ctx)
                        }
                        WsMessage::SubscribeSignatureRequests(params) => {
                            self.subscribe_sig(msg.id, params, ctx)
                        }
                    },
                    Err(error) => {
                        let id = find_id(&text).unwrap_or(-1);
                        error_response(ctx, id, &error);
                    }
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

fn inject_run_id(event: &Event, id: FlowRunId) -> serde_json::Value {
    let mut json = serde_json::to_value(event).unwrap();
    json.get_mut("data")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("flow_run_id".to_owned(), id.to_string().into());
    json
}

impl WsConn {
    fn authenticate(&mut self, id: i64, params: Authenticate, ctx: &mut WebsocketContext<WsConn>) {
        let token = params.token;
        let fut = self
            .auth_service
            .clone()
            .ws_authenticate(token)
            .into_actor(&*self)
            .map(move |res, act, ctx| match res {
                Ok(token) => {
                    act.tokens.insert(token.clone());
                    success_response(ctx, id, token)
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
        if let Some(token) = params.token {
            let fut = self
                .auth_service
                .clone()
                .ws_authenticate(token)
                .into_actor(&*self)
                .map(move |res, act, ctx| match res {
                    Ok(token) => {
                        act.tokens.insert(token.clone());
                        act.subscribe_run(
                            id,
                            SubscribeFlowRunEvents {
                                flow_run_id,
                                token: None,
                            },
                            ctx,
                        );
                    }
                    Err(error) => error_response(ctx, id, &error),
                });
            ctx.wait(fut);
            return;
        }
        let db_worker = self.db_worker.clone();
        let tokens = self.tokens.clone();
        let fut = async move {
            Ok::<_, BoxError>(
                db_worker
                    .send(FindActor::<FlowRunWorker>::new(flow_run_id))
                    .await?
                    .ok_or("not found")?
                    .send(SubscribeEvents { tokens })
                    .await??,
            )
        }
        .into_actor(&*self)
        .map(move |res, act, ctx| match res {
            Ok((stream_id, rx)) => {
                tracing::info!("subscribed flow-run");
                act.subscribing.insert(stream_id);
                success_response(ctx, id, json!({ "stream_id": stream_id }));
                let fut = rx
                    .into_actor(&*act)
                    .map(move |event, _, ctx| {
                        text_stream(ctx, stream_id, inject_run_id(&event, flow_run_id))
                    })
                    .finish()
                    .map(move |_, act, _| {
                        // TODO: send a message indicating stream ended?
                        act.subscribing.remove(&stream_id);
                    });
                ctx.spawn(fut);
            }
            Err(error) => error_response(ctx, id, &error),
        });
        ctx.wait(fut);
    }

    fn subscribe_sig(
        &mut self,
        id: i64,
        _params: SubscribeSignatureRequests,
        ctx: &mut WebsocketContext<WsConn>,
    ) {
        let user_id = self.tokens.iter().find_map(|token| token.user_id());
        let user_id = match user_id {
            Some(user_id) => user_id,
            None => {
                error_response(ctx, id, &"unauthorized");
                return;
            }
        };

        let db_worker = self.db_worker.clone();
        let fut = async move {
            let stream_id = db_worker
                .send(GetUserWorker { user_id })
                .await?
                .send(SubscribeSigReq {})
                .await??;

            Ok::<_, BoxError>(stream_id)
        }
        .into_actor(&*self)
        .map(move |res, act, ctx| match res {
            Ok((stream_id, rx)) => {
                tracing::info!("subscribed signature requests");
                act.subscribing.insert(stream_id);
                success_response(ctx, id, json!({ "stream_id": stream_id }));
                let fut = rx
                    .into_actor(&*act)
                    .map(move |event, _, ctx| {
                        text_stream(
                            ctx,
                            stream_id,
                            json!({
                                "event": "SignatureRequest",
                                "data": event,
                            }),
                        )
                    })
                    .finish()
                    .map(move |_, act, _| {
                        // TODO: send a message indicating stream ended?
                        act.subscribing.remove(&stream_id);
                    });
                ctx.spawn(fut);
            }
            Err(error) => error_response(ctx, id, &error),
        });
        ctx.wait(fut);
    }
}

fn error_response<E: ToString>(ctx: &mut WebsocketContext<WsConn>, id: i64, error: &E) {
    let text = serde_json::to_string(&WsResponse::<()> {
        id,
        data: Err(error.to_string()),
    })
    .unwrap();
    tracing::debug!("sending '{}'", text);
    ctx.text(text);
}

fn success_response<T: Serialize>(ctx: &mut WebsocketContext<WsConn>, id: i64, value: T) {
    let result = serde_json::to_string(&WsResponse::<T> {
        id,
        data: Ok(value),
    });
    match result {
        Ok(text) => {
            tracing::debug!("sending '{}'", text);
            ctx.text(text)
        }
        Err(error) => error_response(
            ctx,
            id,
            &format!("InternalError: failed to serialize event, {error}"),
        ),
    }
}

fn text_stream<T: Serialize>(
    ctx: &mut WebsocketContext<WsConn>,
    stream_id: SubscriptionID,
    event: T,
) {
    let result = serde_json::to_string(&WsEvent::<T> { stream_id, event });
    match result {
        Ok(text) => {
            tracing::debug!("sending '{}'", text);
            ctx.text(text)
        }
        Err(error) => tracing::error!("failed to serialize event: {}", error),
    }
}
