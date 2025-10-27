use flow_lib::context::execute;
use futures::{channel::oneshot, future::BoxFuture};
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_program::hash::Hash;
use solana_rpc_client::nonblocking::rpc_client::RpcClient as SolanaClient;
use solana_rpc_client_api::request::MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS;
use solana_signature::Signature;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;
use tower::Service;

struct Data {
    signature: Signature,
    data: TransactionData,
    sender: oneshot::Sender<Result<Signature, execute::Error>>,
}

#[derive(Clone, Copy)]
pub struct TransactionData {
    pub blockhash: Hash,
    pub slot: u64,
    pub level: CommitmentLevel,
    pub inserted: usize,
}

pub struct Confirmer {
    client: Arc<SolanaClient>,
    need_confirm: Arc<Mutex<Vec<Data>>>,
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
                    let mut query = {
                        let mut data = map.lock().unwrap();
                        let index = data
                            .len()
                            .checked_sub(MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS)
                            .unwrap_or(0);
                        let query = data.split_off(index);
                        query
                    };
                    let sig = query.iter().map(|d| d.signature).collect::<Vec<_>>();
                    let result = client.get_signature_statuses(&sig).await;
                    let mut done = Vec::new();
                    match result {
                        Ok(ok) => {
                            for (index, result) in ok.value.into_iter().enumerate() {
                                match result {
                                    Some(status) => {
                                        if status.satisfies_commitment(CommitmentConfig {
                                            commitment: query[index].data.level,
                                        }) {
                                            done.push((index, status));
                                        }
                                    }
                                    None => {
                                        query[index].data;
                                        // not processed or expired
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            tracing::warn!("{err}");
                        }
                    }
                    for (index, status) in done.into_iter().rev() {
                        let q = query.remove(index);
                        q.sender
                            .send(match status.status {
                                Ok(()) => Ok(q.signature),
                                Err(error) => Err(execute::Error::Solana {
                                    error: Arc::new(error.into()),
                                    inserted: q.data.inserted,
                                }),
                            })
                            .ok();
                    }
                    map.lock().unwrap().extend(query);
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
        self.need_confirm.lock().unwrap().push(Data {
            signature: req.signature,
            data: req.data,
            sender: tx,
        });

        Box::pin(async move { rx.await.map_err(execute::Error::ChannelClosed)? })
    }
}
