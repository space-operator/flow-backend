use crate::{Config, api::flow_api_input::NewRequestService};
use actix::{
    Actor, ActorContext, ActorFutureExt, Arbiter, AsyncContext, Context, ResponseActFuture,
    ResponseFuture, WrapFuture, fut::wrap_future,
};
use command_rpc::flow_side::address_book::BaseAddressBook;
use db::{FlowRunLogsRow, pool::DbPool};
use flow_lib::{
    FlowRunId, UserId,
    config::Endpoints,
    context::{Helius, get_jwt},
    flow_run_events::{DEFAULT_LOG_FILTER, EventSender},
};
use futures_channel::mpsc;
use futures_util::{FutureExt, StreamExt};
use iroh::Watcher;
use n0_watcher::Disconnected;
use serde::Serialize;
use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, LazyLock, atomic::AtomicU64},
    time::Duration,
};
use tokio::sync::broadcast;
use tracing::Span;
use utils::address_book::{AddressBook, AlreadyStarted, ManagableActor};

pub mod flow_run_worker;
pub mod messages;
pub mod signer;
pub mod token_worker;
pub mod user_worker;

pub use flow_run_worker::FlowRunWorker;
pub use user_worker::UserWorker;

use self::{
    token_worker::{LoginWithAdminCred, TokenWorker},
    user_worker::{SubmitError, SubmitSignature},
};

#[derive(Clone, Default)]
pub struct Counter {
    inner: Arc<AtomicU64>,
}

impl Counter {
    pub fn next(&self) -> u64 {
        self.inner
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

pub struct DBWorker {
    db: DbPool,
    endpoints: Endpoints,
    /// All actors in the system
    actors: AddressBook,
    counter: Counter,
    tracing_data: flow_tracing::FlowLogs,
    tx: mpsc::UnboundedSender<Vec<FlowRunLogsRow>>,
    done_tx: broadcast::Sender<()>,
    new_flow_api_request: NewRequestService,
    remote_command_address_book: BaseAddressBook,
    helius: Option<Arc<Helius>>,
}

#[derive(Serialize)]
pub struct IrohInfo {
    pub node_id: String,
    pub relay_url: String,
    pub direct_addresses: Vec<SocketAddr>,
}

pub struct GetIrohInfo;

impl actix::Message for GetIrohInfo {
    type Result = Result<IrohInfo, Disconnected>;
}

impl actix::Handler<GetIrohInfo> for DBWorker {
    type Result = ResponseFuture<<GetIrohInfo as actix::Message>::Result>;

    fn handle(&mut self, _: GetIrohInfo, _: &mut Self::Context) -> Self::Result {
        let endpoint = self.remote_command_address_book.endpoint().clone();
        Box::pin(async move {
            let node_id = endpoint.node_id().to_string();
            let relay_url = endpoint.home_relay().initialized().await.to_string();
            let direct_addresses = endpoint
                .direct_addresses()
                .initialized()
                .await
                .into_iter()
                .map(|addr| addr.addr)
                .collect();

            Ok(IrohInfo {
                node_id,
                relay_url,
                direct_addresses,
            })
        })
    }
}

#[bon::bon]
impl DBWorker {
    #[builder]
    pub fn new(
        db: DbPool,
        config: &Config,
        actors: AddressBook,
        tracing_data: flow_tracing::FlowLogs,
        new_flow_api_request: NewRequestService,
        remote_command_address_book: BaseAddressBook,
        ctx: &mut actix::Context<Self>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded();
        ctx.spawn(
            wrap_future::<_, Self>(db_copy_in(rx, db.clone())).map(|_, act, _| {
                act.done_tx.send(()).ok();
            }),
        );
        let helius = config
            .helius_api_key
            .as_ref()
            .map(|key| Arc::new(Helius::new(crate::HTTP.clone(), key)));

        Self {
            db,
            endpoints: config.endpoints(),
            actors,
            counter: Counter::default(),
            tx,
            tracing_data,
            done_tx: broadcast::channel(1).0,
            new_flow_api_request,
            remote_command_address_book,
            helius,
        }
    }
}

impl actix::SystemService for DBWorker {}

impl actix::Supervised for DBWorker {}

// required in Supervised trait
impl Default for DBWorker {
    fn default() -> Self {
        unimplemented!();
    }
}

impl Actor for DBWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        tracing::info!("started DBWorker");
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::info!("stopped DBWorker");
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SystemShutdown {
    pub timeout: Duration,
}

impl actix::Message for SystemShutdown {
    type Result = ();
}

impl actix::Handler<SystemShutdown> for DBWorker {
    type Result = ResponseActFuture<Self, <SystemShutdown as actix::Message>::Result>;
    fn handle(&mut self, msg: SystemShutdown, _: &mut Self::Context) -> Self::Result {
        let wait = self
            .actors
            .iter::<FlowRunWorker>()
            .map(|(_, addr)| addr.send(msg))
            .collect::<Vec<_>>();
        Box::pin(
            futures_util::future::join_all(wait)
                .into_actor(&*self)
                .then(|_, act, _| {
                    act.tx = mpsc::unbounded().0;
                    let mut rx = act.done_tx.subscribe();
                    async move {
                        rx.recv().await.ok();
                    }
                    .into_actor(&*act)
                })
                .map(|_, _, ctx| ctx.stop()),
        )
    }
}

impl actix::Handler<SubmitSignature> for DBWorker {
    type Result = ResponseFuture<Result<(), SubmitError>>;
    fn handle(&mut self, msg: SubmitSignature, _: &mut Self::Context) -> Self::Result {
        let users = self.actors.iter::<UserWorker>().collect::<Vec<_>>();
        async move {
            for (user_id, addr) in users {
                let res = addr
                    .send(SubmitSignature {
                        user_id,
                        ..msg.clone()
                    })
                    .await;
                match res {
                    Err(_) => continue,
                    Ok(Err(SubmitError::NotFound)) => continue,
                    Ok(Ok(())) => return Ok(()),
                    Ok(Err(error)) => return Err(error),
                }
            }
            Ok(())
        }
        .boxed_local()
    }
}

pub struct GetUserWorker {
    pub user_id: UserId,
    pub base_url: Option<String>,
}

impl actix::Message for GetUserWorker {
    type Result = actix::Addr<UserWorker>;
}

impl actix::Handler<GetUserWorker> for DBWorker {
    type Result = actix::Addr<UserWorker>;
    fn handle(&mut self, msg: GetUserWorker, _: &mut Self::Context) -> Self::Result {
        let id = msg.user_id;
        self.actors.get_or_start(id, {
            let counter = self.counter.clone();
            let db = self.db.clone();
            let endpoints = self.endpoints.clone();
            let new_flow_api_request = self.new_flow_api_request.clone();
            let remote_command_address_book = self.remote_command_address_book.clone();
            let helius = self.helius.clone();
            let arbiter = Arbiter::current();
            move || {
                UserWorker::start_in_arbiter(&arbiter, move |_| {
                    UserWorker::builder()
                        .user_id(id)
                        .endpoints(endpoints)
                        .db(db)
                        .counter(counter)
                        .new_flow_api_request(new_flow_api_request)
                        .remote_command_address_book(remote_command_address_book)
                        .maybe_helius(helius)
                        .build()
                })
            }
        })
    }
}

pub struct GetTokenWorker {
    pub user_id: UserId,
}

impl actix::Message for GetTokenWorker {
    type Result = Result<actix::Addr<TokenWorker>, get_jwt::Error>;
}

impl actix::Handler<GetTokenWorker> for DBWorker {
    type Result = Result<actix::Addr<TokenWorker>, get_jwt::Error>;
    fn handle(&mut self, msg: GetTokenWorker, _: &mut Self::Context) -> Self::Result {
        static HTTP: LazyLock<reqwest::Client> = LazyLock::new(Default::default);

        let id = msg.user_id;
        let addr = self.actors.get_or_start(id, {
            let user_id = msg.user_id;
            let local_db = self.db.get_local().clone();
            let endpoints = self.endpoints.clone();
            let claim = LoginWithAdminCred {
                client: HTTP.clone(),
                user_id,
                db: self.db.clone(),
                endpoints: endpoints.clone(),
            };
            move || {
                TokenWorker::start_in_arbiter(&Arbiter::current(), move |_| {
                    TokenWorker::new(user_id, local_db, endpoints, claim)
                })
            }
        });
        Ok(addr)
    }
}

pub struct RegisterLogs {
    pub flow_run_id: FlowRunId,
    pub tx: EventSender,
    pub filter: Option<String>,
}

impl actix::Message for RegisterLogs {
    type Result = Result<Span, Infallible>;
}

impl actix::Handler<RegisterLogs> for DBWorker {
    type Result = <RegisterLogs as actix::Message>::Result;
    fn handle(&mut self, msg: RegisterLogs, _: &mut Self::Context) -> Self::Result {
        Ok(self.tracing_data.register_flow_logs(
            msg.flow_run_id,
            msg.filter.as_deref().unwrap_or(DEFAULT_LOG_FILTER),
            msg.tx,
        ))
    }
}

pub struct StartFlowRunWorker<F>
where
    F: FnOnce(&mut Context<FlowRunWorker>) -> FlowRunWorker + Send + 'static,
{
    pub id: <FlowRunWorker as ManagableActor>::ID,
    pub make_actor: F,
}

impl<F> actix::Message for StartFlowRunWorker<F>
where
    F: FnOnce(&mut Context<FlowRunWorker>) -> FlowRunWorker + Send + 'static,
{
    type Result = Result<actix::Addr<FlowRunWorker>, AlreadyStarted>;
}

impl<F> actix::Handler<StartFlowRunWorker<F>> for DBWorker
where
    F: FnOnce(&mut Context<FlowRunWorker>) -> FlowRunWorker + Send + 'static,
{
    type Result = Result<actix::Addr<FlowRunWorker>, AlreadyStarted>;

    fn handle(&mut self, msg: StartFlowRunWorker<F>, _: &mut Self::Context) -> Self::Result {
        self.actors
            .try_start_with_context(msg.id, msg.make_actor, Arbiter::current())
    }
}

pub struct FindActor<A: ManagableActor> {
    pub id: A::ID,
}

impl<A: ManagableActor> actix::Message for FindActor<A> {
    type Result = Option<actix::Addr<A>>;
}

impl<A: ManagableActor> FindActor<A> {
    pub fn new(id: A::ID) -> Self {
        Self { id }
    }
}

impl actix::Handler<FindActor<FlowRunWorker>> for DBWorker {
    type Result = Option<actix::Addr<FlowRunWorker>>;

    fn handle(&mut self, msg: FindActor<FlowRunWorker>, _: &mut Self::Context) -> Self::Result {
        self.actors
            .get::<FlowRunWorker>(msg.id)
            .and_then(|weak| weak.upgrade())
    }
}

impl actix::Handler<FindActor<UserWorker>> for DBWorker {
    type Result = Option<actix::Addr<UserWorker>>;

    fn handle(&mut self, msg: FindActor<UserWorker>, _: &mut Self::Context) -> Self::Result {
        self.actors
            .get::<UserWorker>(msg.id)
            .and_then(|weak| weak.upgrade())
    }
}

pub struct CopyIn<T>(pub T);

impl<T> actix::Message for CopyIn<T> {
    type Result = ();
}

impl actix::Handler<CopyIn<Vec<FlowRunLogsRow>>> for DBWorker {
    type Result = ();

    fn handle(&mut self, msg: CopyIn<Vec<FlowRunLogsRow>>, _: &mut Self::Context) -> Self::Result {
        self.tx.unbounded_send(msg.0).ok();
    }
}

async fn db_copy_in(rx: mpsc::UnboundedReceiver<Vec<FlowRunLogsRow>>, db: DbPool) {
    const CHUNK_SIZE: usize = 16;
    let mut chunks = rx.ready_chunks(CHUNK_SIZE);

    while let Some(events) = chunks.next().await {
        let conn = match db.get_admin_conn().await {
            Ok(conn) => conn,
            Err(error) => {
                tracing::error!(
                    "could not get DB connection, dropping events. detail: {}",
                    error
                );
                continue;
            }
        };
        let res = conn
            .copy_in_flow_run_logs(events.iter().flat_map(|vec| vec.iter()))
            .await;
        match res {
            Ok(count) => tracing::debug!("inserted {} rows", count),
            Err(error) => tracing::error!("{}, dropping events.", error),
        }
    }
}
