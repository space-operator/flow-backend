use actix::{Actor, ActorFutureExt, AsyncContext, Context, ResponseFuture, WrapFuture};
use actix_web::{dev::ServerHandle, web, App, HttpServer};
use futures_channel::oneshot;
use hashbrown::HashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::marker::PhantomData;
use thiserror::Error as ThisError;
use tower::{util::BoxService, BoxError, Service, ServiceBuilder, ServiceExt};
use url::Url;

pub type JsonService = BoxService<JsonValue, JsonValue, JsonValue>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request<T = JsonValue> {
    pub envelope: String,
    pub svc_name: String,
    pub svc_id: String,
    pub input: T,
}

impl actix::Message for Request {
    type Result = Result<Response, Error>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response<T = JsonValue> {
    pub envelope: String,
    pub success: bool,
    pub data: T,
}

pub struct RegisterJsonService<S, T> {
    pub name: String,
    pub id: String,
    pub service: S,
    _phantom: PhantomData<T>,
}

impl<S, T> RegisterJsonService<S, T> {
    pub fn new(name: String, id: String, service: S) -> Self {
        Self {
            name,
            id,
            service,
            _phantom: <_>::default(),
        }
    }
}

pub struct RegisterServiceResult {
    pub old_service: Option<JsonService>,
    pub name: String,
    pub id: String,
    pub base_url: Url,
}

impl<S, T> actix::Message for RegisterJsonService<S, T> {
    type Result = RegisterServiceResult;
}

pub struct RemoveService {
    pub name: String,
    pub id: String,
}

impl actix::Message for RemoveService {
    type Result = Option<JsonService>;
}

pub struct Server {
    /// svc_name => (svc_id => S)
    services: HashMap<String, HashMap<String, JsonService>>,
    port: u16,
    server_handle: ServerHandle,
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("service not found: {}", .0)]
    NotFound(String),
    #[error("service dropped without sending a response")]
    Dropped,
    #[error("bind error: {}", .0)]
    Bind(std::io::Error),
}

impl Actor for Server {
    type Context = Context<Self>;

    fn stopped(&mut self, _: &mut Self::Context) {
        actix::spawn(self.server_handle.stop(true));
    }
}

impl Server {
    pub fn start_http_server() -> Result<actix::Addr<Self>, Error> {
        let ctx = Context::<Self>::new();
        let addr = ctx.address();
        let server = HttpServer::new(move || {
            App::new()
                .wrap(actix_web::middleware::Logger::new(r#""%r" %s %b %Dms"#))
                .configure(|s| configure_server(s, addr.downgrade()))
        })
        .workers(1)
        .bind("127.0.0.1:0")
        .map_err(Error::Bind)?;
        let port = server.addrs()[0].port();
        let server = server.run();
        let server_handle = server.handle();
        actix::spawn(server);
        Ok(ctx.run(Self {
            services: <_>::default(),
            server_handle,
            port,
        }))
    }

    pub fn base_url(&self) -> Url {
        Url::parse(&format!("http://127.0.0.1:{}", self.port)).unwrap()
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
                    .insert(req.svc_id, s);
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
            .ok_or_else(|| Error::NotFound(format!("svc_name: {}", req.svc_name)))?
            .remove(&req.svc_id)
            .ok_or_else(|| Error::NotFound(format!("svc_id: {}", req.svc_id)))?;

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
        tracing::info!("inserting {}::{}", name, id);
        let s = ServiceBuilder::new()
            .filter(|r: JsonValue| {
                tracing::debug!("request: {}", r);
                serde_json::from_value::<T>(r)
            })
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
            .map_result(|r| {
                match &r {
                    Ok(x) => tracing::debug!("success: {}", x),
                    Err(x) => tracing::debug!("error: {}", x),
                }
                r
            })
            .boxed();
        self.services.entry(name).or_default().insert(id, s)
    }
}

impl actix::Handler<Request> for Server {
    type Result = ResponseFuture<<Request as actix::Message>::Result>;
    fn handle(&mut self, msg: Request, ctx: &mut Self::Context) -> Self::Result {
        let r = self.process_request(ctx, msg);

        Box::pin(async move { r?.await.map_err(|_| Error::Dropped) })
    }
}

impl<S, T> actix::Handler<RegisterJsonService<S, T>> for Server
where
    S: tower::Service<T> + Send + 'static,
    T: DeserializeOwned,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Response: Serialize,
    S::Future: Send + 'static,
{
    type Result = actix::Response<<RegisterJsonService<S, T> as actix::Message>::Result>;
    fn handle(&mut self, msg: RegisterJsonService<S, T>, _: &mut Self::Context) -> Self::Result {
        let old_service = self.register_json_service(msg.name.clone(), msg.id.clone(), msg.service);
        actix::Response::reply(RegisterServiceResult {
            old_service,
            name: msg.name,
            id: msg.id,
            base_url: self.base_url(),
        })
    }
}

impl actix::Handler<RemoveService> for Server {
    type Result = actix::Response<<RemoveService as actix::Message>::Result>;
    fn handle(&mut self, msg: RemoveService, _: &mut Self::Context) -> Self::Result {
        tracing::info!("removing {}::{}", msg.name, msg.id);
        actix::Response::reply(
            self.services
                .get_mut(&msg.name)
                .and_then(|map| map.remove(&msg.id)),
        )
    }
}

pub fn configure_server(s: &mut web::ServiceConfig, addr: actix::WeakAddr<Server>) {
    async fn call(
        body: web::Json<Request>,
        addr: web::Data<actix::WeakAddr<Server>>,
    ) -> web::Json<Response> {
        let req = body.into_inner();
        let envelope = req.envelope.clone();

        let addr = match addr.upgrade() {
            Some(addr) => addr,
            None => {
                return web::Json(Response {
                    envelope,
                    success: false,
                    data: "Server stopped".into(),
                })
            }
        };

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
