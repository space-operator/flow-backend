use std::{
    collections::BTreeSet,
    future::{Ready, ready},
};

use command_rpc::flow_side::address_book::authenticate;
use futures_util::future::{self, BoxFuture};
use tower::Service;

use crate::middleware::auth_v1;

#[derive(Clone, bon::Builder)]
pub struct WorkerAuthenticate {
    trusted: BTreeSet<iroh::PublicKey>,
    auth: auth_v1::AuthV1,
}

impl Service<authenticate::Request> for WorkerAuthenticate {
    type Response = authenticate::Response;

    type Error = authenticate::Error;

    type Future = future::Either<
        Ready<Result<Self::Response, Self::Error>>,
        BoxFuture<'static, Result<Self::Response, Self::Error>>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: authenticate::Request) -> Self::Future {
        if self.trusted.contains(&req.pubkey) {
            return future::Either::Left(ready(Ok(authenticate::Response {})));
        }
        if let Some(apikey) = req.apikey {
            let auth = self.auth.clone();
            return future::Either::Right(Box::pin(async move {
                auth.apikey_authenticate(&apikey).await?;
                Ok(authenticate::Response {})
            }));
        }
        future::Either::Left(ready(Err(anyhow::anyhow!("failed"))))
    }
}
