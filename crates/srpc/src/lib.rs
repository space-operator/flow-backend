use actix::{Actor, ActorFutureExt, AsyncContext, Context, ResponseFuture, WrapFuture};
use actix_web::web;
use futures_channel::oneshot;
use hashbrown::HashMap;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use serde_tuple::*;
use thiserror::Error as ThisError;
use tower::{util::BoxService, BoxError, Service, ServiceBuilder, ServiceExt};

pub use smartstring::alias::String;

#[derive(Serialize_tuple, Deserialize_tuple, Debug, Clone)]
pub struct Request {
    pub envelope: String,
    pub svc_name: String,
    pub svc_id: String,
    pub input: JsonValue,
}

impl actix::Message for Request {
    type Result = Result<Response, Error>;
}

#[derive(Serialize_tuple, Deserialize_tuple, Debug, Clone)]
pub struct Response {
    pub envelope: String,
    pub success: bool,
    pub data: JsonValue,
}

pub struct Server {
    /// svc_name => (svc_id => S)
    services: HashMap<String, HashMap<String, BoxService<JsonValue, JsonValue, JsonValue>>>,
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("not found")]
    NotFound,
    #[error("service dropped without sending a response")]
    Dropped,
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Server {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    fn after_ready(
        &mut self,
        result: Result<BoxService<JsonValue, JsonValue, JsonValue>, JsonValue>,
        req: Request,
        responder: oneshot::Sender<Response>,
    ) {
        match result {
            Ok(mut s) => {
                let future = s.call(req.input);
                actix::spawn(async move {
                    let (success, data) = match future.await {
                        Ok(x) => (true, x),
                        Err(x) => (false, x),
                    };
                    responder
                        .send(Response {
                            envelope: req.envelope,
                            success,
                            data,
                        })
                        .ok();
                });
                self.services
                    .get_mut(&req.svc_name)
                    .unwrap()
                    .insert(req.svc_id, s)
                    .unwrap();
            }
            Err(error) => {
                let _ = responder.send(Response {
                    envelope: req.envelope,
                    success: false,
                    data: error,
                });
            }
        }
    }

    fn process_request(
        &mut self,
        ctx: &mut actix::Context<Self>,
        req: Request,
    ) -> Result<oneshot::Receiver<Response>, Error> {
        let mut s = self
            .services
            .get_mut(&req.svc_name)
            .ok_or(Error::NotFound)?
            .remove(&req.svc_id)
            .ok_or(Error::NotFound)?;

        let (tx, rx) = oneshot::channel();
        let task = async move {
            s.ready().await?;
            Ok(s)
        }
        .into_actor(&*self)
        .map(move |result, actor, _| actor.after_ready(result, req, tx));
        ctx.spawn(task);
        Ok(rx)
    }

    pub fn register_json_service<S, T>(
        &mut self,
        name: String,
        id: String,
        s: S,
    ) -> Option<BoxService<JsonValue, JsonValue, JsonValue>>
    where
        S: tower::Service<T> + Send + 'static,
        T: DeserializeOwned,
        S::Error: std::error::Error + Send + Sync + 'static,
        S::Response: Serialize,
        S::Future: Send + 'static,
    {
        let s = ServiceBuilder::new()
            .filter(|r: JsonValue| serde_json::from_value::<T>(r))
            .map_result(
                |r: Result<S::Response, S::Error>| -> Result<JsonValue, BoxError> {
                    match r {
                        Ok(t) => serde_json::to_value(&t).map_err(|e| e.into()),
                        Err(e) => Err(e.into()),
                    }
                },
            )
            .check_service::<S, JsonValue, JsonValue, BoxError>()
            .service(s)
            .map_err(|error| JsonValue::String(error.to_string()))
            .boxed();
        self.services.entry(name).or_default().insert(id, s)
    }
}

impl actix::Handler<Request> for Server {
    type Result = ResponseFuture<Result<Response, Error>>;
    fn handle(&mut self, msg: Request, ctx: &mut Self::Context) -> Self::Result {
        let r = self.process_request(ctx, msg);

        Box::pin(async move { Ok(r?.await.map_err(|_| Error::Dropped)?) })
    }
}

pub fn configure_server(addr: actix::Addr<Server>, s: &mut web::ServiceConfig) {
    async fn call(
        body: web::Json<Request>,
        addr: web::Data<actix::Addr<Server>>,
    ) -> web::Json<Response> {
        let req = body.into_inner();
        let envelope = req.envelope.clone();
        let result = addr.send(req).await;

        web::Json(match result {
            Ok(result) => match result {
                Ok(resp) => resp,
                Err(error) => Response {
                    envelope,
                    success: false,
                    data: JsonValue::String(error.to_string()),
                },
            },
            Err(error) => Response {
                envelope,
                success: false,
                data: JsonValue::String(error.to_string()),
            },
        })
    }
    s.app_data(web::Data::new(addr))
        .route("/call", web::post().to(call));
}
