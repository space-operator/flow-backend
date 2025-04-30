use super::prelude::*;
use crate::db_worker::{FindActor, FlowRunWorker};
use actix_web::web::ServiceConfig;
use chrono::Utc;
use flow::flow_run_events::ApiInput;
use flow_lib::{config::Endpoints, context::api_input, utils::tower_client::CommonErrorExt};
use futures_channel::oneshot;
use futures_util::future::BoxFuture;
use std::{sync::Mutex, time::Duration};

struct Responder {
    oneshot: oneshot::Sender<Result<api_input::Response, api_input::Error>>,
}

pub struct RequestStore {
    reqs: ahash::HashMap<blake3::Hash, Responder>,
    secret: [u8; blake3::KEY_LEN],
}

impl RequestStore {
    fn new() -> Self {
        Self {
            reqs: <_>::default(),
            secret: rand::random(),
        }
    }

    pub fn new_app_data() -> web::Data<Mutex<Self>> {
        web::Data::new(Mutex::new(Self::new()))
    }

    pub fn make_key(&self, req: &api_input::Request) -> blake3::Hash {
        let mut h = blake3::Hasher::new_keyed(&self.secret);
        h.update(req.flow_run_id.as_bytes());
        h.update(req.node_id.as_bytes());
        h.update(&req.times.to_le_bytes());
        h.finalize()
    }
}

#[derive(Clone)]
pub struct NewRequestService {
    pub store: web::Data<Mutex<RequestStore>>,
    pub db_worker: actix::Addr<DBWorker>,
    pub endpoints: Endpoints,
}

impl tower::Service<api_input::Request> for NewRequestService {
    type Response = api_input::Response;

    type Error = api_input::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: api_input::Request) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        let flow_run_id = req.flow_run_id;

        let mut store = self.store.lock().unwrap();
        let key = store.make_key(&req);
        let timeout = req.timeout;
        store.reqs.insert(key, Responder { oneshot: tx });
        let url = format!(
            "{}/flow/submit/{}",
            self.endpoints.flow_server,
            key.to_hex(),
        );
        let db_worker = self.db_worker.clone();
        Box::pin(async move {
            match db_worker
                .send(FindActor::<FlowRunWorker>::new(flow_run_id))
                .await
            {
                Ok(Some(addr)) => {
                    addr.do_send(ApiInput {
                        time: Utc::now(),
                        url,
                    });
                }
                _ => {
                    return Err(api_input::Error::msg(format!(
                        "flow is not running: {}",
                        flow_run_id
                    )));
                }
            };
            if timeout != Duration::MAX {
                tokio::time::timeout(timeout, rx)
                    .await
                    .map_err(|_| api_input::Error::Timeout)?
                    .map_err(|_| api_input::Error::Timeout)?
            } else {
                rx.await.map_err(|_| api_input::Error::Timeout)?
            }
        })
    }
}

pub fn configure(store: web::Data<Mutex<RequestStore>>) -> impl FnOnce(&mut ServiceConfig) {
    move |app: &mut ServiceConfig| {
        app.app_data(store)
            .service(web::resource("/submit/{key}").post(submit_data));
    }
}

#[derive(Deserialize)]
struct Body {
    value: value::Value,
}

async fn submit_data(
    path: web::Path<String>,
    store: web::Data<Mutex<RequestStore>>,
    body: web::Json<Body>,
) -> Result<web::Json<()>, actix_web::Error> {
    let key = blake3::Hash::from_hex(path.into_inner()).map_err(|_| Error::NotFound)?;
    if let Some(resp) = store.lock().unwrap().reqs.remove(&key) {
        resp.oneshot
            .send(Ok(api_input::Response {
                value: body.into_inner().value,
            }))
            .map_err(|_| Error::NotFound)?;
        Ok(web::Json(()))
    } else {
        return Err(Error::NotFound.into());
    }
}
