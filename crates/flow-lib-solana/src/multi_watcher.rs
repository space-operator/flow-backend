use flow_lib::context::execute;
use futures::{channel::oneshot, future::BoxFuture};
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_program::hash::Hash;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::request::{MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS, RpcError};
use solana_signature::Signature;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{sync::Notify, task::JoinHandle};
use tower::{Service, ServiceExt};

struct Data {
    signature: Signature,
    data: TransactionData,
    sender: oneshot::Sender<Result<Signature, execute::Error>>,
}

#[derive(Clone, Copy)]
pub struct TransactionData {
    pub blockhash: Hash,
    pub last_valid_block_height: u64,
    pub level: CommitmentLevel,
    pub inserted: usize,
}

pub struct Confirmer {
    client: Arc<RpcClient>,
    need_confirm: Arc<Mutex<Vec<Data>>>,
    task: Option<JoinHandle<()>>,
    notify: Arc<Notify>,
    blockhash_data_svc: CacheService<BlockhashService>,
}

pub struct Confirm {
    pub signature: Signature,
    pub data: TransactionData,
}

#[derive(Clone)]
struct BlockhashData {
    current_block_height: u64,
}

#[derive(Clone)]
struct BlockhashService {
    client: Arc<RpcClient>,
}

impl Service<()> for BlockhashService {
    type Response = BlockhashData;

    type Error = solana_rpc_client_api::client_error::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: ()) -> Self::Future {
        let client = self.client.clone();
        Box::pin(async move {
            let current_block_height = client
                .get_block_height_with_commitment(CommitmentConfig::processed())
                .await?;

            Ok(BlockhashData {
                current_block_height,
            })
        })
    }
}

#[derive(Clone)]
struct CacheService<S>
where
    S: Service<()>,
{
    time: Duration,
    value: Arc<Mutex<Option<S::Response>>>,
    fetch_time: Arc<Mutex<Option<Instant>>>,
    service: S,
}

impl<S> Service<()> for CacheService<S>
where
    S: Service<(), Response: Clone + Send + 'static, Error: Send + 'static, Future: Send + 'static>,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ()) -> Self::Future {
        let mut fetch_time = self.fetch_time.lock().unwrap();
        if let Some(instant) = *fetch_time {
            if instant.elapsed() > self.time {
                *fetch_time = None;
                *self.value.lock().unwrap() = None;
            }
        }

        if let Some(value) = self.value.lock().unwrap().clone() {
            Box::pin(async move { Ok(value) })
        } else {
            let fut = self.service.call(req);
            let fetch_time = self.fetch_time.clone();
            let value = self.value.clone();
            Box::pin(async move {
                let result = fut.await;
                match result {
                    Ok(result) => {
                        *value.lock().unwrap() = Some(result.clone());
                        *fetch_time.lock().unwrap() = Some(Instant::now());
                        Ok(result)
                    }
                    Err(error) => Err(error),
                }
            })
        }
    }
}

impl Confirmer {
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self {
            client: client.clone(),
            need_confirm: Default::default(),
            task: None,
            notify: Default::default(),
            blockhash_data_svc: CacheService {
                time: Duration::from_secs(5),
                value: Default::default(),
                fetch_time: Default::default(),
                service: BlockhashService { client },
            },
        }
    }

    fn spawn(&mut self) {
        if self.task.is_none() {
            let map = self.need_confirm.clone();
            let client = self.client.clone();
            let notify = self.notify.clone();
            let mut svc = self.blockhash_data_svc.clone();
            self.task = Some(tokio::spawn(async move {
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
                    if query.is_empty() {
                        notify.notified().await;
                        continue;
                    }

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
                                            done.push((index, Some(status)));
                                        }
                                    }
                                    None => {
                                        let current_block_height = svc
                                            .ready()
                                            .await
                                            .unwrap()
                                            .call(())
                                            .await
                                            .unwrap()
                                            .current_block_height;
                                        let is_expired = query[index].data.last_valid_block_height
                                            < current_block_height;
                                        if is_expired {
                                            done.push((index, None));
                                        }
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
                            .send(match status {
                                None => Err(execute::Error::Solana {
                                    error: Arc::new(RpcError::ForUser("unable to confirm transaction.\
                                                   This can happen in situations such as transaction expiration
                                                   and insufficient fee-payer funds".to_owned()).into()),
                                    inserted: q.data.inserted,
                                }),
                                Some(status) => match status.status {
                                    Ok(()) => Ok(q.signature),
                                    Err(error) => Err(execute::Error::Solana {
                                        error: Arc::new(error.into()),
                                        inserted: q.data.inserted,
                                    }),
                                },
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
        self.notify.notify_one();

        Box::pin(async move { rx.await.map_err(execute::Error::ChannelClosed)? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{
        StreamExt,
        channel::mpsc,
        future::{join, join_all},
    };
    use rand::seq::IndexedRandom;
    use solana_keypair::Keypair;
    use solana_rpc_client_api::config::RpcSendTransactionConfig;
    use solana_signer::Signer;
    use tower::util::CallAllUnordered;

    #[tokio::test]
    async fn test_confirm_need_key() {
        let from =
            Keypair::from_base58_string(&std::env::var("TEST_CONFIRM_KEYPAIR_FROM").unwrap());
        let to = Keypair::from_base58_string(&std::env::var("TEST_CONFIRM_KEYPAIR_TO").unwrap());
        let client = Arc::new(RpcClient::new(std::env::var("SOLANA_URL").unwrap()));

        let confirmer = Confirmer::new(client.clone());
        let count = 30;
        let (hash, height) = client
            .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
            .await
            .unwrap();
        let (sender, receiver) = mpsc::unbounded();
        let tasks = (0..count).map(|i| {
            let client = client.clone();
            let sender = sender.clone();
            let tx = solana_system_transaction::transfer(&from, &to.pubkey(), 100000 + i, hash);
            async move {
                tokio::time::sleep(Duration::from_secs_f64(rand::random_range(0.1..2.0))).await;
                match client
                    .send_transaction_with_config(
                        &tx,
                        RpcSendTransactionConfig {
                            skip_preflight: true,
                            preflight_commitment: Some(CommitmentLevel::Finalized),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    Ok(signature) => {
                        sender
                            .unbounded_send(Confirm {
                                signature,
                                data: TransactionData {
                                    blockhash: hash,
                                    last_valid_block_height: height,
                                    level: *[
                                        CommitmentLevel::Confirmed,
                                        CommitmentLevel::Finalized,
                                    ]
                                    .choose(&mut rand::rng())
                                    .unwrap(),
                                    inserted: 0,
                                },
                            })
                            .ok();
                    }
                    Err(error) => {
                        println!("{error}");
                    }
                }
            }
        });
        let (_, _) = join(
            CallAllUnordered::new(confirmer, receiver).for_each(async |result| {
                let _ = dbg!(result);
            }),
            async {
                join_all(tasks).await;
                sender.close_channel();
            },
        )
        .await;

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
