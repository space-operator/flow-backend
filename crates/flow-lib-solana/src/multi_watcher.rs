use flow_lib::context::execute;
use futures::{channel::oneshot, future::BoxFuture};
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_program::hash::Hash;
use solana_rpc_client::nonblocking::rpc_client::RpcClient as SolanaClient;
use solana_signature::Signature;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;
use tower::Service;

struct Data {
    data: TransactionData,
    sender: oneshot::Sender<Result<Signature, execute::Error>>,
}

#[derive(Clone, Copy)]
pub struct TransactionData {
    pub blockhash: Hash,
    pub slot: u64,
    pub level: CommitmentLevel,
}

pub struct Confirmer {
    client: Arc<SolanaClient>,
    need_confirm: Arc<Mutex<BTreeMap<Signature, Data>>>,
    task: Option<JoinHandle<()>>,
}

pub struct Confirm {
    pub signature: Signature,
    pub data: TransactionData,
}

impl Confirmer {
    fn spawn(&mut self) {
        if self.task.is_none() {
            let map = self.need_confirm.clone();
            let client = self.client.clone();
            self.task = Some(tokio::spawn(async move {
                // wait for a tx

                loop {
                    let (sig, data) = {
                        let map = map.lock().unwrap();
                        let (sig, data): (Vec<_>, Vec<_>) =
                            map.iter().map(|(k, v)| (*k, v.data.clone())).unzip();
                        (sig, data)
                    };
                    let result = client.get_signature_statuses(&sig).await;
                    match result {
                        Ok(ok) => {
                            for (index, result) in ok.value.into_iter().enumerate() {
                                match result {
                                    Some(status) => {
                                        if status.satisfies_commitment(CommitmentConfig {
                                            commitment: data[index].level,
                                        }) {}
                                    }
                                    None => {}
                                }
                            }
                        }
                        Err(err) => {
                            tracing::warn!("{err}");
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }));
        }
    }
}

impl Service<Confirm> for Confirmer {
    type Response = Signature;

    type Error = execute::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.spawn();
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Confirm) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        self.need_confirm.lock().unwrap().insert(
            req.signature,
            Data {
                data: req.data,
                sender: tx,
            },
        );

        Box::pin(async move { rx.await.map_err(execute::Error::ChannelClosed)? })
    }
}
