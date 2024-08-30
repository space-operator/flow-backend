use crate::{flow_logs, Config};
use actix::{
    fut::wrap_future, Actor, ActorContext, ActorFutureExt, Arbiter, AsyncContext, Context,
    ResponseActFuture, ResponseFuture, WrapFuture,
};
use db::{
    pool::{DbPool, ProxiedDbPool, RealDbPool},
    FlowRunLogsRow,
};
use flow::flow_run_events::{EventSender, DEFAULT_LOG_FILTER, FLOW_SPAN_NAME};
use flow_lib::{config::Endpoints, context::get_jwt, BoxError, FlowRunId, UserId};
use futures_channel::mpsc;
use futures_util::{FutureExt, StreamExt};
use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
use tokio::sync::broadcast;
use tracing::{level_filters::LevelFilter, Span};
use tracing_subscriber::EnvFilter;
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
    tracing_data: flow_logs::Map,
    tx: mpsc::UnboundedSender<Vec<FlowRunLogsRow>>,
    done_tx: broadcast::Sender<()>,
}

impl DBWorker {
    pub fn new(
        db: DbPool,
        config: Config,
        actors: AddressBook,
        tracing_data: flow_logs::Map,
        ctx: &mut actix::Context<Self>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded();
        match &db {
            DbPool::Real(db) => ctx.spawn(wrap_future::<_, Self>(db_copy_in(rx, db.clone())).map(
                |_, act, _| {
                    act.done_tx.send(()).ok();
                },
            )),
            DbPool::Proxied(db) => ctx.spawn(
                wrap_future::<_, Self>(db_copy_in_proxied(rx, db.clone())).map(|_, act, _| {
                    act.done_tx.send(()).ok();
                }),
            ),
        };

        Self {
            db,
            endpoints: config.endpoints(),
            actors,
            counter: Counter::default(),
            tx,
            tracing_data,
            done_tx: broadcast::channel(1).0,
        }
    }
}

impl Actor for DBWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        tracing::info!("started DBWorker");
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::warn!("stopped DBWorker");
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
}

impl actix::Message for GetUserWorker {
    type Result = actix::Addr<UserWorker>;
}

impl actix::Handler<GetUserWorker> for DBWorker {
    type Result = actix::Addr<UserWorker>;
    fn handle(&mut self, msg: GetUserWorker, ctx: &mut Self::Context) -> Self::Result {
        let id = msg.user_id;
        self.actors.get_or_start(id, {
            let counter = self.counter.clone();
            let db = self.db.clone();
            let root = ctx.address();
            let endpoints = self.endpoints.clone();
            move || {
                UserWorker::start_in_arbiter(&Arbiter::current(), move |_| {
                    UserWorker::new(id, endpoints, db, counter, root)
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
        let id = msg.user_id;
        match &self.db {
            DbPool::Real(db) => {
                let addr = self.actors.get_or_start(id, {
                    let user_id = msg.user_id;
                    let local_db = self.db.get_local().clone();
                    let endpoints = self.endpoints.clone();
                    let claim = LoginWithAdminCred {
                        client: reqwest::Client::new(),
                        user_id,
                        db: db.clone(),
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
            DbPool::Proxied(_) => self
                .actors
                .get::<TokenWorker>(id)
                .ok_or(get_jwt::Error::NotAllowed)?
                .upgrade()
                .ok_or(get_jwt::Error::other("TokenWorker stopped")),
        }
    }
}

pub struct RegisterLogs {
    pub flow_run_id: FlowRunId,
    pub tx: EventSender,
    pub filter: Option<String>,
}

impl actix::Message for RegisterLogs {
    type Result = Result<Span, BoxError>;
}

impl actix::Handler<RegisterLogs> for DBWorker {
    type Result = <RegisterLogs as actix::Message>::Result;
    fn handle(&mut self, msg: RegisterLogs, _: &mut Self::Context) -> Self::Result {
        let span = tracing::error_span!(FLOW_SPAN_NAME, flow_run_id = msg.flow_run_id.to_string());
        let id = span.id().ok_or("span ID is None")?;
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::ERROR.into())
            .parse_lossy(msg.filter.as_deref().unwrap_or(DEFAULT_LOG_FILTER));
        let mut map = self.tracing_data.write().unwrap();
        map.insert(id, flow_logs::Data { tx: msg.tx, filter });
        Ok(span)
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

async fn db_copy_in(rx: mpsc::UnboundedReceiver<Vec<FlowRunLogsRow>>, db: RealDbPool) {
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

async fn db_copy_in_proxied(rx: mpsc::UnboundedReceiver<Vec<FlowRunLogsRow>>, db: ProxiedDbPool) {
    const CHUNK_SIZE: usize = 16;
    let mut chunks = rx.ready_chunks(CHUNK_SIZE);
    while let Some(events) = chunks.next().await {
        let rows = events
            .into_iter()
            .flat_map(|vec| vec.into_iter())
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        let user_id = rows[0].user_id;
        let conn = match db.get_user_conn(user_id).await {
            Ok(conn) => conn,
            Err(error) => {
                tracing::error!(
                    "could not get DB connection, dropping events. detail: {}",
                    error
                );
                continue;
            }
        };
        let count = rows.len();
        let res = conn.push_logs(&rows).await;
        match res {
            Ok(_) => tracing::debug!("inserted {} rows", count),
            Err(error) => tracing::error!("{}, dropping events.", error),
        }
    }
}
