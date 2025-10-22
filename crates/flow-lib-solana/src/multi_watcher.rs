use flow_lib::context::execute;
use futures::future::BoxFuture;
use solana_commitment_config::CommitmentLevel;
use solana_rpc_client::nonblocking::rpc_client::RpcClient as SolanaClient;
use solana_signature::Signature;
use std::sync::Arc;
use tower::Service;

pub struct Confirmer {
    _client: Arc<SolanaClient>,
}

pub struct Confirm {
    pub signature: Signature,
    pub level: CommitmentLevel,
}

impl Service<Confirm> for Confirmer {
    type Response = Signature;

    type Error = execute::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _req: Confirm) -> Self::Future {
        todo!()
    }
}
