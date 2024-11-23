use super::{
    flow_run_worker::FlowRunWorker, messages::SubscribeError, signer::SignerWorker, Counter,
    DBWorker, FindActor, GetTokenWorker, RegisterLogs, StartFlowRunWorker,
};
use crate::error::ErrorBody;
use actix::{
    Actor, ActorFutureExt, AsyncContext, Response, ResponseActFuture, ResponseFuture, WrapFuture,
};
use actix_web::{http::StatusCode, ResponseError};
use bytes::Bytes;
use db::{pool::DbPool, Error as DbError};
use flow::{
    flow_graph::StopSignal,
    flow_registry::{get_flow, get_previous_values, new_flow_run, FlowRegistry, StartFlowOptions},
};
use flow_lib::{
    config::{
        client::{FlowRunOrigin, PartialConfig},
        Endpoints,
    },
    context::{
        env::RUST_LOG,
        get_jwt,
        signer::{self, SignatureRequest},
    },
    solana::{is_same_message_logic, Pubkey, SolanaActionConfig},
    FlowId, FlowRunId, User, UserId,
};
use futures_channel::{mpsc, oneshot};
use futures_util::{future::BoxFuture, TryFutureExt};
use hashbrown::HashMap;
use solana_sdk::signature::Signature;
use std::future::ready;
use thiserror::Error as ThisError;
use utils::address_book::ManagableActor;

pub struct UserWorker {
    root: actix::Addr<DBWorker>,
    db: DbPool,
    counter: Counter,
    user_id: UserId,
    endpoints: Endpoints,
    sigreg: HashMap<i64, SigReq>,
    subs: HashMap<u64, Subscription>,
}

pub struct SubscribeSigReq {}

impl actix::Message for SubscribeSigReq {
    type Result = Result<(u64, mpsc::UnboundedReceiver<SignatureRequest>), SubscribeError>;
}

impl actix::Handler<SubscribeSigReq> for UserWorker {
    type Result = <SubscribeSigReq as actix::Message>::Result;

    fn handle(&mut self, _msg: SubscribeSigReq, _: &mut Self::Context) -> Self::Result {
        let stream_id = self.counter.next();
        let (tx, rx) = mpsc::unbounded();
        self.subs.insert(stream_id, Subscription { tx });
        Ok((stream_id, rx))
    }
}

struct Subscription {
    tx: mpsc::UnboundedSender<SignatureRequest>,
}

#[derive(Clone)]
pub struct SigReqEvent(pub SignatureRequest);

impl actix::Message for SigReqEvent {
    type Result = ();
}

#[derive(Debug)]
struct SigReq {
    resp: oneshot::Sender<Result<signer::SignatureResponse, signer::Error>>,
    req: signer::SignatureRequest,
}

impl ManagableActor for UserWorker {
    type ID = UserId;

    fn id(&self) -> Self::ID {
        self.user_id
    }
}

impl Actor for UserWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        tracing::debug!("started UserWorker {}", self.user_id);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::Running {
        self.subs.retain(|_, v| !v.tx.is_closed());
        if self.subs.is_empty() {
            actix::Running::Stop
        } else {
            actix::Running::Continue
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::debug!("stopped UserWorker {}", self.user_id);
    }
}

impl UserWorker {
    pub fn new(
        user_id: UserId,
        endpoints: Endpoints,
        db: DbPool,
        counter: Counter,
        root: actix::Addr<DBWorker>,
    ) -> Self {
        Self {
            user_id,
            endpoints,
            db,
            counter,
            root,
            sigreg: <_>::default(),
            subs: <_>::default(),
        }
    }

    fn process_sigreq(
        &mut self,
        result: Result<(i64, signer::SignatureRequest), DbError>,
        ctx: &mut actix::Context<Self>,
    ) -> BoxFuture<'static, Result<signer::SignatureResponse, signer::Error>> {
        match result {
            Ok((id, mut req)) => {
                req.id = Some(id);
                let (tx, rx) = oneshot::channel();
                let timeout = req.timeout;
                self.sigreg
                    .try_insert(
                        id,
                        SigReq {
                            resp: tx,
                            req: req.clone(),
                        },
                    )
                    .expect("DB's ID is unique");
                self.subs
                    .retain(|_, sub| sub.tx.unbounded_send(req.clone()).is_ok());
                if let Some(flow_run_id) = req.flow_run_id {
                    actix::spawn(
                        self.root
                            .send(FindActor::<FlowRunWorker>::new(flow_run_id))
                            .map_ok(move |res| {
                                if let Some(addr) = res {
                                    addr.do_send(SigReqEvent(req.clone()));
                                }
                            })
                            .inspect_err(move |_| {
                                tracing::error!("FlowRunWorker not found {}", flow_run_id);
                            }),
                    );
                }
                ctx.run_later(timeout, move |act, _| {
                    if let Some(SigReq { resp, .. }) = act.sigreg.remove(&id) {
                        resp.send(Err(signer::Error::Timeout)).ok();
                    }
                });
                Box::pin(async move {
                    rx.await
                        .map_err(|_| signer::Error::Other("tx dropped".into()))?
                })
            }
            Err(error) => Box::pin(ready(Err(signer::Error::Other(error.into())))),
        }
    }
}

pub struct SigReqExists {
    pub id: i64,
}

impl actix::Message for SigReqExists {
    type Result = bool;
}

impl actix::Handler<SigReqExists> for UserWorker {
    type Result = Response<<SigReqExists as actix::Message>::Result>;

    fn handle(&mut self, msg: SigReqExists, _: &mut Self::Context) -> Self::Result {
        Response::reply(self.sigreg.contains_key(&msg.id))
    }
}

impl actix::Handler<get_previous_values::Request> for UserWorker {
    type Result = ResponseFuture<Result<get_previous_values::Response, get_previous_values::Error>>;

    fn handle(&mut self, msg: get_previous_values::Request, _: &mut Self::Context) -> Self::Result {
        let user_id = self.user_id;
        let db = self.db.clone();
        let fut = async move {
            if user_id != msg.user_id {
                return Err(get_previous_values::Error::Unauthorized);
            }

            let values = db
                .get_user_conn(user_id)
                .await
                .map_err(|e| get_previous_values::Error::Other(e.into()))?
                .get_previous_values(&msg.nodes)
                .await
                .map_err(|e| get_previous_values::Error::Other(e.into()))?;

            Ok(get_previous_values::Response { values })
        };

        Box::pin(fut)
    }
}

impl actix::Handler<get_flow::Request> for UserWorker {
    type Result = ResponseFuture<Result<get_flow::Response, get_flow::Error>>;

    fn handle(&mut self, msg: get_flow::Request, _: &mut Self::Context) -> Self::Result {
        let user_id = self.user_id;
        let db = self.db.clone();
        let fut = async move {
            if user_id != msg.user_id {
                return Err(get_flow::Error::Unauthorized);
            }

            let config = db
                .get_user_conn(user_id)
                .await
                .map_err(|e| get_flow::Error::Other(e.into()))?
                .get_flow_config(msg.flow_id)
                .await
                .map_err(|e| match e {
                    DbError::ResourceNotFound { .. } => get_flow::Error::NotFound,
                    _ => get_flow::Error::Other(e.into()),
                })?;

            Ok(get_flow::Response { config })
        };

        Box::pin(fut)
    }
}

impl actix::Handler<new_flow_run::Request> for UserWorker {
    type Result = ResponseFuture<Result<new_flow_run::Response, new_flow_run::Error>>;

    fn handle(&mut self, msg: new_flow_run::Request, _: &mut Self::Context) -> Self::Result {
        let user_id = self.user_id;
        let db = self.db.clone();
        let root = self.root.clone();
        let counter = self.counter.clone();
        Box::pin(async move {
            if user_id != msg.user_id {
                return Err(new_flow_run::Error::Unauthorized);
            }

            let conn = db
                .get_user_conn(user_id)
                .await
                .map_err(new_flow_run::Error::other)?;
            let run_id = conn
                .new_flow_run(&msg.config, &msg.inputs)
                .await
                .map_err(new_flow_run::Error::other)?;

            for id in &msg.shared_with {
                if *id != user_id {
                    conn.share_flow_run(run_id, *id)
                        .await
                        .map_err(new_flow_run::Error::other)?;
                }
            }

            let stop_signal = StopSignal::new();
            let stop_shared_signal = StopSignal::new();

            root.send(StartFlowRunWorker {
                id: run_id,
                make_actor: {
                    let stop_signal = stop_signal.clone();
                    let stop_shared_signal = stop_shared_signal.clone();
                    let root = root.clone();
                    move |ctx| {
                        FlowRunWorker::new(
                            run_id,
                            user_id,
                            msg.shared_with,
                            counter,
                            msg.stream,
                            db,
                            root.clone(),
                            stop_signal.clone(),
                            stop_shared_signal.clone(),
                            ctx,
                        )
                    }
                },
            })
            .await?
            .map_err(|_| new_flow_run::Error::Other("could not start worker".into()))?;

            let span = root
                .send(RegisterLogs {
                    flow_run_id: run_id,
                    tx: msg.tx,
                    filter: msg.config.environment.get(RUST_LOG).cloned(),
                })
                .await?
                .map_err(new_flow_run::Error::Other)?;

            Ok(new_flow_run::Response {
                flow_run_id: run_id,
                stop_signal,
                stop_shared_signal,
                span,
            })
        })
    }
}

impl actix::Handler<signer::SignatureRequest> for UserWorker {
    type Result = ResponseActFuture<Self, Result<signer::SignatureResponse, signer::Error>>;

    fn handle(&mut self, msg: signer::SignatureRequest, _: &mut Self::Context) -> Self::Result {
        let db = self.db.clone();
        let user_id = self.user_id;
        async move {
            let id = db
                .get_user_conn(user_id)
                .await?
                .new_signature_request(
                    &msg.pubkey.to_bytes(),
                    &msg.message,
                    msg.flow_run_id.as_ref(),
                    msg.signatures.as_deref(),
                )
                .await?;
            Ok((id, msg))
        }
        .into_actor(&*self)
        .then(move |result, act, ctx| act.process_sigreq(result, ctx).into_actor(act))
        .boxed_local()
    }
}

#[derive(Clone, Debug)]
pub struct SubmitSignature {
    pub user_id: UserId,
    pub id: i64,
    pub signature: [u8; 64],
    pub new_msg: Option<Bytes>,
}

impl actix::Message for SubmitSignature {
    type Result = Result<(), SubmitError>;
}

#[derive(ThisError, Debug)]
pub enum SubmitError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("could not update tx because it will invalidate existing signature")]
    NotAllowChangeTx,
    #[error("transaction changed: {}", .0)]
    TxChanged(anyhow::Error),
    #[error("signature verification failed")]
    WrongSignature,
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Mailbox(#[from] actix::MailboxError),
}

impl ResponseError for SubmitError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubmitError::Unauthorized => StatusCode::UNAUTHORIZED,
            SubmitError::NotFound => StatusCode::NOT_FOUND,
            SubmitError::NotAllowChangeTx => StatusCode::BAD_REQUEST,
            SubmitError::WrongSignature => StatusCode::BAD_REQUEST,
            SubmitError::TxChanged(_) => StatusCode::BAD_REQUEST,
            SubmitError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SubmitError::Mailbox(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        ErrorBody::build(self)
    }
}

impl actix::Handler<SubmitSignature> for UserWorker {
    type Result = ResponseFuture<Result<(), SubmitError>>;

    fn handle(&mut self, mut msg: SubmitSignature, _: &mut Self::Context) -> Self::Result {
        if self.user_id != msg.user_id {
            return Box::pin(ready(Err(SubmitError::Unauthorized)));
        }
        if !self.sigreg.contains_key(&msg.id) {
            return Box::pin(ready(Err(SubmitError::NotFound)));
        }
        let req = self.sigreg.remove(&msg.id).unwrap();
        let mut message = req.req.message.clone();
        if let Some(new) = msg.new_msg {
            if new == req.req.message {
                msg.new_msg = None;
            } else {
                if !req
                    .req
                    .signatures
                    .as_ref()
                    .map(|s| s.is_empty())
                    .unwrap_or(true)
                {
                    return Box::pin(ready(Err(SubmitError::NotAllowChangeTx)));
                }
                if let Err(error) = is_same_message_logic(&req.req.message, &new) {
                    self.sigreg.insert(msg.id, req);
                    return Box::pin(ready(Err(SubmitError::TxChanged(error))));
                }
                msg.new_msg = Some(new.clone());
                message = new;
            }
        }
        if !Signature::from(msg.signature).verify(&req.req.pubkey.to_bytes(), &message) {
            self.sigreg.insert(msg.id, req);
            return Box::pin(ready(Err(SubmitError::WrongSignature)));
        }
        let db = self.db.clone();
        let user_id = self.user_id;
        req.resp
            .send(Ok(signer::SignatureResponse {
                signature: Signature::from(msg.signature),
                new_message: msg.new_msg.clone(),
            }))
            .ok();
        Box::pin(async move {
            db.get_user_conn(user_id)
                .await?
                .save_signature(&msg.id, &msg.signature, msg.new_msg.as_ref())
                .await?;

            Ok(())
        })
    }
}

pub struct StartFlowFresh {
    pub user: User,
    pub flow_id: FlowId,
    pub input: value::Map,
    pub output_instructions: bool,
    pub action_identity: Option<Pubkey>,
    pub action_config: Option<SolanaActionConfig>,
    pub fees: Vec<(Pubkey, u64)>,
    pub partial_config: Option<PartialConfig>,
    pub environment: HashMap<String, String>,
}

#[derive(ThisError, Debug)]
pub enum StartError {
    #[error("unauthorized")]
    Unauthorized,
    #[error(transparent)]
    Flow(#[from] flow::Error),
    #[error(transparent)]
    NewFlowRun(#[from] new_flow_run::Error),
    #[error(transparent)]
    Jwt(#[from] get_jwt::Error),
    #[error(transparent)]
    Mailbox(#[from] actix::MailboxError),
    #[error(transparent)]
    Db(#[from] db::Error),
}

impl ResponseError for StartError {
    fn status_code(&self) -> StatusCode {
        match self {
            StartError::Unauthorized => StatusCode::NOT_FOUND,
            StartError::Flow(e) => match e {
                flow::Error::Any(_) => StatusCode::INTERNAL_SERVER_ERROR,
                flow::Error::Canceled(_) => StatusCode::INTERNAL_SERVER_ERROR,
                flow::Error::ValueNotFound(_) => StatusCode::INTERNAL_SERVER_ERROR,
                flow::Error::CreateCmd(_) => StatusCode::INTERNAL_SERVER_ERROR,
                flow::Error::BuildGraphError(_) => StatusCode::BAD_REQUEST,
                flow::Error::GetFlow(e) => match e {
                    get_flow::Error::NotFound => StatusCode::NOT_FOUND,
                    get_flow::Error::Unauthorized => StatusCode::UNAUTHORIZED,
                    get_flow::Error::InvalidInferflow { .. }
                    | get_flow::Error::Worker(_)
                    | get_flow::Error::MailBox(_)
                    | get_flow::Error::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
                },
                flow::Error::Cycle => StatusCode::BAD_REQUEST,
                flow::Error::NeedOneTx => StatusCode::BAD_REQUEST,
                flow::Error::NeedOneSignatureOutput => StatusCode::BAD_REQUEST,
            },
            StartError::NewFlowRun(e) => match e {
                new_flow_run::Error::BuildFlow(_) | new_flow_run::Error::GetPreviousValues(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
                new_flow_run::Error::NotFound => StatusCode::NOT_FOUND,
                new_flow_run::Error::Unauthorized => StatusCode::UNAUTHORIZED,
                new_flow_run::Error::MaxDepthReached => StatusCode::BAD_REQUEST,
                new_flow_run::Error::Worker(_)
                | new_flow_run::Error::MailBox(_)
                | new_flow_run::Error::Other(_) => StatusCode::UNAUTHORIZED,
            },
            StartError::Jwt(e) => match e {
                get_jwt::Error::NotAllowed | get_jwt::Error::UserNotFound => {
                    StatusCode::UNAUTHORIZED
                }
                get_jwt::Error::WrongRecipient
                | get_jwt::Error::Worker(_)
                | get_jwt::Error::MailBox(_)
                | get_jwt::Error::Supabase { .. }
                | get_jwt::Error::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            StartError::Mailbox(_) => StatusCode::INTERNAL_SERVER_ERROR,
            StartError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        ErrorBody::build(self)
    }
}

impl actix::Message for StartFlowFresh {
    type Result = Result<FlowRunId, StartError>;
}

impl actix::Handler<StartFlowFresh> for UserWorker {
    type Result = ResponseFuture<Result<FlowRunId, StartError>>;

    fn handle(&mut self, msg: StartFlowFresh, ctx: &mut Self::Context) -> Self::Result {
        let user_id = self.user_id;
        let addr = ctx.address();
        let endpoints = self.endpoints.clone();
        let root = self.root.clone();
        let db = self.db.clone();
        Box::pin(async move {
            if msg.user.id != user_id {
                return Err(StartError::Unauthorized);
            }

            let wrk = root.send(GetTokenWorker { user_id }).await??;

            let (signer, signers_info) =
                SignerWorker::fetch_and_start(db, &[(user_id, addr.clone().recipient())]).await?;

            let r = FlowRegistry::from_actix(
                msg.user,
                msg.user,
                Vec::new(),
                msg.flow_id,
                (signer.recipient(), signers_info),
                addr.clone().recipient(),
                addr.clone().recipient(),
                addr.clone().recipient(),
                wrk.recipient(),
                msg.environment,
                endpoints,
            )
            .await?;

            let run_id = r
                .start(
                    msg.flow_id,
                    msg.input,
                    StartFlowOptions {
                        partial_config: msg.partial_config,
                        collect_instructions: msg.output_instructions,
                        action_identity: msg.action_identity,
                        action_config: msg.action_config,
                        fees: msg.fees,
                        origin: FlowRunOrigin::Start {},
                        ..Default::default()
                    },
                )
                .await?
                .0;

            Ok(run_id)
        })
    }
}

pub struct StartFlowShared {
    pub flow_id: FlowId,
    pub input: value::Map,
    pub output_instructions: bool,
    pub action_identity: Option<Pubkey>,
    pub action_config: Option<SolanaActionConfig>,
    pub fees: Vec<(Pubkey, u64)>,
    pub started_by: (UserId, actix::Addr<UserWorker>),
}

impl actix::Message for StartFlowShared {
    type Result = Result<FlowRunId, StartError>;
}

impl actix::Handler<StartFlowShared> for UserWorker {
    type Result = ResponseFuture<<StartFlowShared as actix::Message>::Result>;

    fn handle(&mut self, msg: StartFlowShared, ctx: &mut Self::Context) -> Self::Result {
        if msg.started_by.0 == self.user_id {
            return self.handle(
                StartFlowFresh {
                    user: User { id: self.user_id },
                    flow_id: msg.flow_id,
                    input: msg.input,
                    output_instructions: msg.output_instructions,
                    action_identity: msg.action_identity,
                    action_config: msg.action_config,
                    fees: msg.fees,
                    partial_config: None,
                    environment: <_>::default(),
                },
                ctx,
            );
        }

        let user_id = self.user_id;
        let addr = ctx.address();
        let endpoints = self.endpoints.clone();
        let root = self.root.clone();
        let db = self.db.clone();
        Box::pin(async move {
            let wrk = root.send(GetTokenWorker { user_id }).await??;

            let (signer, signers_info) = SignerWorker::fetch_and_start(
                db,
                &[
                    (msg.started_by.0, msg.started_by.1.recipient()),
                    (user_id, addr.clone().recipient()),
                ],
            )
            .await?;

            let r = FlowRegistry::from_actix(
                User { id: user_id },
                User {
                    id: msg.started_by.0,
                },
                [msg.started_by.0].into(),
                msg.flow_id,
                (signer.recipient(), signers_info),
                addr.clone().recipient(),
                addr.clone().recipient(),
                addr.clone().recipient(),
                wrk.recipient(),
                <_>::default(),
                endpoints,
            )
            .await?;

            let run_id = r
                .start(
                    msg.flow_id,
                    msg.input,
                    StartFlowOptions {
                        collect_instructions: msg.output_instructions,
                        action_identity: msg.action_identity,
                        action_config: msg.action_config,
                        origin: FlowRunOrigin::StartShared {
                            started_by: msg.started_by.0,
                        },
                        fees: msg.fees,
                        ..Default::default()
                    },
                )
                .await?
                .0;

            Ok(run_id)
        })
    }
}

#[derive(Clone, Copy)]
pub struct CloneFlow {
    pub user_id: UserId,
    pub flow_id: FlowId,
}

#[derive(ThisError, Debug)]
pub enum CloneFlowError {
    #[error("unauthorized")]
    Unauthorized,
    #[error(transparent)]
    Db(#[from] DbError),
}

impl ResponseError for CloneFlowError {
    fn status_code(&self) -> StatusCode {
        match self {
            CloneFlowError::Unauthorized => StatusCode::NOT_FOUND,
            CloneFlowError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        ErrorBody::build(self)
    }
}

impl actix::Message for CloneFlow {
    type Result = Result<HashMap<FlowId, FlowId>, CloneFlowError>;
}

impl actix::Handler<CloneFlow> for UserWorker {
    type Result = ResponseFuture<Result<HashMap<FlowId, FlowId>, CloneFlowError>>;

    fn handle(&mut self, msg: CloneFlow, _: &mut Self::Context) -> Self::Result {
        let db = self.db.clone();
        let user_id = self.user_id;

        let fut = async move {
            if user_id != msg.user_id {
                return Err(CloneFlowError::Unauthorized);
            }

            Ok(db
                .get_user_conn(user_id)
                .await?
                .clone_flow(msg.flow_id)
                .await?)
        };

        Box::pin(fut)
    }
}
