use std::sync::Mutex;

use actix_web::web::ServiceConfig;
use flow_lib::{
    context::api_input,
    utils::{TowerClient, tower_client::CommonErrorExt},
};
use futures_channel::oneshot;
use futures_util::future::BoxFuture;

use super::prelude::*;

struct Responder {
    oneshot: oneshot::Sender<Result<api_input::Response, api_input::Error>>,
}

#[derive(Default)]
pub struct RequestStore {
    reqs: ahash::HashMap<api_input::Request, Responder>,
}

#[derive(Clone)]
pub struct Service {
    store: web::Data<Mutex<RequestStore>>,
}

impl tower::Service<api_input::Request> for Service {
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
    let service = TowerClient::new(Service {
        store: store.clone(),
    });
    app.app_data(service);
}
