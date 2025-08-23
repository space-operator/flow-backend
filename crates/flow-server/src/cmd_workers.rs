use std::{
    collections::BTreeSet,
    future::{Ready, ready},
};

use command_rpc::flow_side::address_book::authenticate;
use tower::Service;

#[derive(Debug, Clone, bon::Builder)]
pub struct WorkerAuthenticate {
    trusted: BTreeSet<iroh::PublicKey>,
}

impl Service<authenticate::Request> for WorkerAuthenticate {
    type Response = authenticate::Response;

    type Error = authenticate::Error;

    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: authenticate::Request) -> Self::Future {
        if self.trusted.contains(&req) {
            ready(Ok(authenticate::Response {}))
        } else {
            ready(Err(anyhow::anyhow!("failed")))
        }
    }
}
