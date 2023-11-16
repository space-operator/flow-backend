use crate::Config;
use actix::{Actor, ActorContext, ActorFutureExt, ArbiterHandle, AsyncContext, WrapFuture};
use db::{
    pool::{DbPool, ProxiedDbPool, RealDbPool},
    FlowRunLogsRow,
};
use flow_lib::{config::Endpoints, context::get_jwt, UserId};
use futures_channel::mpsc;
use futures_util::StreamExt;
use std::sync::{atomic::AtomicU64, Arc};
use utils::address_book::{AddressBook, ManagableActor};

pub mod flow_run_worker;
pub mod messages;
pub mod signer;
pub mod token_worker;
pub mod user_worker;

pub use flow_run_worker::FlowRunWorker;
pub use user_worker::UserWorker;

use self::token_worker::{LoginWithAdminCred, TokenWorker};

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
    tx: Option<mpsc::UnboundedSender<Vec<FlowRunLogsRow>>>,
}

impl DBWorker {
    pub fn new(db: DbPool, config: Config, actors: AddressBook) -> Self {
        Self {
            db,
            endpoints: config.endpoints(),
            actors,
            counter: Counter::default(),
            tx: None,
        }
    }
}

impl Actor for DBWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("started DBWorker");
        if self.tx.is_none() {
            let (tx, rx) = mpsc::unbounded();
            self.tx = Some(tx);
            match &self.db {
                DbPool::Real(db) => ctx.spawn(
                    db_copy_in(rx, db.clone())
                        .into_actor(&*self)
                        .map(|_, _, ctx| ctx.stop()),
                ),
                DbPool::Proxied(db) => ctx.spawn(
                    db_copy_in_proxied(rx, db.clone())
                        .into_actor(&*self)
                        .map(|_, _, ctx| ctx.stop()),
                ),
            };
        } else {
            tracing::error!("started called twice");
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::warn!("stopped DBWorker");
    }
}

pub struct GetUserWorker {
    pub user_id: UserId,
    pub rt: ArbiterHandle,
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
                UserWorker::start_in_arbiter(&msg.rt, move |_| {
                    UserWorker::new(id, endpoints, db, counter, root)
                })
            }
        })
    }
}

pub struct GetTokenWorker {
    pub user_id: UserId,
    pub rt: ArbiterHandle,
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
                        TokenWorker::start_in_arbiter(&msg.rt, move |_| {
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

pub struct StartActor<A: ManagableActor> {
    pub actor: A,
    pub rt: ArbiterHandle,
}

impl<A: ManagableActor> actix::Message for StartActor<A> {
    type Result = Result<actix::Addr<A>, A>;
}

impl actix::Handler<StartActor<FlowRunWorker>> for DBWorker {
    type Result = Result<actix::Addr<FlowRunWorker>, FlowRunWorker>;

    fn handle(&mut self, msg: StartActor<FlowRunWorker>, _: &mut Self::Context) -> Self::Result {
        self.actors.try_start_in_rt(msg.actor, msg.rt)
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
        let opt = self
            .tx
            .as_ref()
            .and_then(move |tx| tx.unbounded_send(msg.0).ok());
        if opt.is_none() {
            tracing::error!("channel closed");
        }
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
