use super::prelude::*;
use actix_web::web::ServiceConfig;
use flow_lib::{NodeId, context::api_input, utils::TowerClient};
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

#[derive(Clone)]
pub struct NewRequestService {
    store: web::Data<Mutex<RequestStore>>,
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
        self.store
            .lock()
            .unwrap()
            .reqs
            .insert(req, Responder { oneshot: tx });
        Box::pin(async move { rx.await.expect("we never drop this channel") })
    }
}

pub fn configure(app: &mut ServiceConfig) {
    let store = web::Data::new(Mutex::new(RequestStore::default()));
    let service = TowerClient::new(NewRequestService {
        store: store.clone(),
    });
    app.app_data(service)
        .app_data(store)
        .service(web::resource("/submit/{flow_run_id}/{node_id}/{times}").post(submit_data));
}

async fn submit_data(
    path: web::Path<(FlowRunId, NodeId, u32)>,
    store: web::Data<Mutex<RequestStore>>,
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
                value: value::Value::Null,
            }))
            .map_err(|_| Error::NotFound)?;
        Ok(web::Json(()))
    } else {
        return Err(Error::NotFound.into());
    }
}
