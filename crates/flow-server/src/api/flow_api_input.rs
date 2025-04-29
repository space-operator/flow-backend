use crate::db_worker::{FindActor, FlowRunWorker};

use super::prelude::*;
use actix_web::web::ServiceConfig;
use chrono::Utc;
use flow::flow_run_events::ApiInput;
use flow_lib::{NodeId, config::Endpoints, context::api_input};
use futures_channel::oneshot;
use futures_util::future::BoxFuture;
use std::sync::Mutex;

struct Responder {
    oneshot: oneshot::Sender<Result<api_input::Response, api_input::Error>>,
}

#[derive(Default)]
pub struct RequestStore {
    reqs: ahash::HashMap<api_input::Request, Responder>,
}

impl RequestStore {
    pub fn new_app_data() -> web::Data<Mutex<Self>> {
        web::Data::new(Mutex::new(Self::default()))
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
        let url = format!(
            "{}/flow/submit/{}/{}/{}",
            self.endpoints.flow_server, req.flow_run_id, req.node_id, req.times
        );
        self.store
            .lock()
            .unwrap()
            .reqs
            .insert(req, Responder { oneshot: tx });
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
                    tracing::warn!("flow is not running: {}", flow_run_id);
                }
            };
            rx.await.expect("we never drop this channel")
        })
    }
}

pub fn configure(store: web::Data<Mutex<RequestStore>>) -> impl FnOnce(&mut ServiceConfig) {
    move |app: &mut ServiceConfig| {
        app.app_data(store)
            .service(web::resource("/submit/{flow_run_id}/{node_id}/{times}").post(submit_data));
    }
}

#[derive(Deserialize)]
struct Body {
    value: value::Value,
}

async fn submit_data(
    path: web::Path<(FlowRunId, NodeId, u32)>,
    store: web::Data<Mutex<RequestStore>>,
    body: web::Json<Body>,
) -> Result<web::Json<()>, actix_web::Error> {
    let (flow_run_id, node_id, times) = path.into_inner();
    let req = api_input::Request {
        flow_run_id,
        node_id,
        times,
    };
    if let Some(resp) = store.lock().unwrap().reqs.remove(&req) {
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
