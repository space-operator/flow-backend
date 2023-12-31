use super::{
    messages::{Finished, SubscribeError, SubscriptionID},
    user_worker::SigReqEvent,
    CopyIn, Counter, DBWorker, SystemShutdown,
};
use crate::error::ErrorBody;
use actix::{
    fut::wrap_future, Actor, ActorContext, ActorFutureExt, AsyncContext, ResponseActFuture,
    StreamHandler, WrapFuture,
};
use actix_web::http::StatusCode;
use db::{pool::DbPool, FlowRunLogsRow};
use flow::{
    flow_graph::StopSignal,
    flow_run_events::{
        self, Event, FlowError, FlowFinish, FlowLog, FlowStart, NodeError, NodeFinish, NodeLog,
        NodeOutput, NodeStart,
    },
};
use flow_lib::{FlowRunId, UserId};
use futures_channel::mpsc;
use futures_util::{stream::BoxStream, StreamExt};
use hashbrown::HashMap;
use thiserror::Error as ThisError;
use tokio::sync::broadcast;
use utils::address_book::ManagableActor;
use value::Value;

pub struct FlowRunWorker {
    user_id: UserId,
    shared_with: Vec<UserId>,
    run_id: FlowRunId,
    stop_signal: StopSignal,
    stop_shared_signal: StopSignal,
    counter: Counter,
    tx: mpsc::UnboundedSender<Event>,
    subs: HashMap<SubscriptionID, Subscription>,
    all_events: Vec<Event>,
    done_tx: broadcast::Sender<()>,
}

impl Actor for FlowRunWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        tracing::info!("started FlowRunWorker {}", self.run_id);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::info!("stopped FlowRunWorker {}", self.run_id);
        self.stop_signal.stop(0);
    }
}

impl ManagableActor for FlowRunWorker {
    type ID = FlowRunId;

    fn id(&self) -> Self::ID {
        self.run_id
    }
}

impl actix::Handler<SigReqEvent> for FlowRunWorker {
    type Result = ();
    fn handle(&mut self, msg: SigReqEvent, _: &mut Self::Context) -> Self::Result {
        self.subs.retain(|id, sub| {
            let retain = if let Some(addr) = sub.receiver1.upgrade() {
                addr.do_send(SigReqEvent {
                    sub_id: *id,
                    ..msg.clone()
                });
                true
            } else {
                false
            };
            retain
        });
    }
}

impl actix::Handler<SystemShutdown> for FlowRunWorker {
    type Result = ResponseActFuture<Self, <SystemShutdown as actix::Message>::Result>;
    fn handle(&mut self, msg: SystemShutdown, _: &mut Self::Context) -> Self::Result {
        let mut rx = self.done_tx.subscribe();
        let stop_signal = self.stop_signal.clone();
        let id = self.run_id;
        Box::pin(
            async move {
                let res = tokio::time::timeout(msg.timeout, rx.recv()).await;
                if res.is_err() {
                    tracing::warn!("force stopping FlowRunWorker {}", id);
                    stop_signal.stop(0);
                    rx.recv().await.ok();
                }
            }
            .into_actor(&*self)
            .map(|_, _, ctx| ctx.stop()),
        )
    }
}

struct Subscription {
    finished: actix::WeakRecipient<Finished>,
    receiver: actix::WeakRecipient<FullEvent>,
    receiver1: actix::WeakRecipient<SigReqEvent>,
}

pub struct FullEvent {
    pub sub_id: SubscriptionID,
    pub flow_run_id: FlowRunId,
    pub event: Event,
}

impl actix::Message for FullEvent {
    type Result = ();
}

pub struct SubscribeEvents {
    pub user_id: UserId,
    pub flow_run_id: FlowRunId,
    pub finished: actix::WeakRecipient<Finished>,
    pub receiver: actix::WeakRecipient<FullEvent>,
    pub receiver1: actix::WeakRecipient<SigReqEvent>,
}

impl actix::Message for SubscribeEvents {
    type Result = Result<(SubscriptionID, Vec<Event>), SubscribeError>;
}

impl actix::Handler<SubscribeEvents> for FlowRunWorker {
    type Result = Result<(SubscriptionID, Vec<Event>), SubscribeError>;

    fn handle(&mut self, msg: SubscribeEvents, _: &mut Self::Context) -> Self::Result {
        if !msg.user_id.is_nil()
            && msg.user_id != self.user_id
            && !self.shared_with.contains(&msg.user_id)
        {
            return Err(SubscribeError::Unauthorized {
                user_id: msg.user_id,
            });
        }
        msg.receiver
            .upgrade()
            .ok_or(SubscribeError::MailBox(actix::MailboxError::Closed))?;
        let sub_id = self.counter.next();
        self.subs.insert(
            sub_id,
            Subscription {
                finished: msg.finished,
                receiver: msg.receiver,
                receiver1: msg.receiver1,
            },
        );
        Ok((sub_id, self.all_events.clone()))
    }
}

pub struct StopFlow {
    pub user_id: UserId,
    pub run_id: FlowRunId,
    pub timeout_millies: u32,
}

impl actix::Message for StopFlow {
    type Result = Result<(), StopError>;
}

#[derive(ThisError, Debug)]
pub enum StopError {
    #[error("unauthorized: {}", user_id)]
    Unauthorized { user_id: UserId },
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Mailbox(#[from] actix::MailboxError),
    #[error(transparent)]
    Worker(#[from] flow_lib::BoxError),
}

impl actix_web::ResponseError for StopError {
    fn status_code(&self) -> StatusCode {
        match self {
            StopError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            StopError::NotFound => StatusCode::NOT_FOUND,
            StopError::Mailbox(_) => StatusCode::INTERNAL_SERVER_ERROR,
            StopError::Worker(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        ErrorBody::build(self)
    }
}

impl actix::Handler<StopFlow> for FlowRunWorker {
    type Result = Result<(), StopError>;

    fn handle(&mut self, msg: StopFlow, _: &mut Self::Context) -> Self::Result {
        if self.user_id != msg.user_id {
            if self.shared_with.contains(&msg.user_id) {
                self.stop_shared_signal.stop(msg.timeout_millies);
                return Ok(());
            }
            return Err(StopError::Unauthorized {
                user_id: msg.user_id,
            });
        }
        if self.run_id != msg.run_id {
            return Err(StopError::NotFound);
        }
        self.stop_signal.stop(msg.timeout_millies);
        Ok(())
    }
}

impl FlowRunWorker {
    pub fn new(
        run_id: FlowRunId,
        user_id: UserId,
        shared_with: Vec<UserId>,
        counter: Counter,
        stream: BoxStream<'static, flow_run_events::Event>,
        db: DbPool,
        root: actix::Addr<DBWorker>,
        stop_signal: StopSignal,
        stop_shared_signal: StopSignal,
        ctx: &mut actix::Context<Self>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded();
        let fut = save_to_db(user_id, run_id, rx, db, root.recipient());
        ctx.spawn(wrap_future::<_, Self>(fut).map(move |_, act, _| {
            act.done_tx.send(()).ok();
        }));
        ctx.add_stream(stream);

        FlowRunWorker {
            user_id,
            shared_with,
            run_id,
            stop_signal,
            stop_shared_signal,
            counter,
            tx,
            done_tx: broadcast::channel::<()>(1).0,
            subs: HashMap::new(),
            all_events: Vec::new(),
        }
    }

    pub fn stop_signal(&self) -> StopSignal {
        self.stop_signal.clone()
    }

    pub fn stop_shared_signal(&self) -> StopSignal {
        self.stop_shared_signal.clone()
    }
}

impl StreamHandler<Event> for FlowRunWorker {
    fn handle(&mut self, item: Event, _: &mut Self::Context) {
        let is_finished = matches!(&item, Event::FlowFinish(_));

        self.tx.unbounded_send(item.clone()).ok();
        if is_finished {
            self.tx.close_channel();
        }

        self.subs.retain(|id, sub| {
            let retain = if let Some(addr) = sub.receiver.upgrade() {
                addr.do_send(FullEvent {
                    sub_id: *id,
                    flow_run_id: self.run_id,
                    event: item.clone(),
                });
                true
            } else {
                false
            };
            if is_finished {
                if let Some(addr) = sub.finished.upgrade() {
                    addr.do_send(Finished { sub_id: *id });
                }
            }
            retain
        });
        self.all_events.push(item);
    }

    fn finished(&mut self, _: &mut Self::Context) {
        self.tx.close_channel();
    }
}

fn log_error<E: std::fmt::Display>(error: E) {
    tracing::error!("{}, dropping event.", error);
}

/// Max 16 KB for each fields
const MAX_SIZE: usize = 32 * 1024;

/// Strip long values to save data
fn strip(value: Value) -> Value {
    match value {
        Value::String(s) if s.len() > MAX_SIZE => "VALUE TOO LARGE".into(),
        Value::Bytes(s) if s.len() > MAX_SIZE => "VALUE TOO LARGE".into(),
        Value::Array(mut s) => {
            for v in &mut s {
                *v = strip(std::mem::take(v));
            }
            Value::Array(s)
        }
        Value::Map(mut s) => {
            for v in s.values_mut() {
                *v = strip(std::mem::take(v));
            }
            Value::Map(s)
        }
        _ => value,
    }
}

async fn save_to_db(
    user_id: UserId,
    run_id: FlowRunId,
    rx: mpsc::UnboundedReceiver<Event>,
    db: DbPool,
    tx: actix::Recipient<CopyIn<Vec<FlowRunLogsRow>>>,
) {
    let mut log_index = 0i32;
    const CHUNK_SIZE: usize = 16;
    let mut chunks = rx.ready_chunks(CHUNK_SIZE);
    while let Some(events) = chunks.next().await {
        let mut logs: Vec<FlowRunLogsRow> = Vec::new();
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
        for event in events {
            match event {
                Event::FlowStart(FlowStart { time }) => {
                    conn.set_start_time(&run_id, &time)
                        .await
                        .map_err(log_error)
                        .ok();
                }
                Event::FlowError(FlowError { error, .. }) => {
                    conn.push_flow_error(&run_id, error.as_str())
                        .await
                        .map_err(log_error)
                        .ok();
                }
                Event::FlowFinish(FlowFinish {
                    time,
                    not_run,
                    output,
                }) => {
                    conn.set_run_result(&run_id, &time, &not_run, &output)
                        .await
                        .map_err(log_error)
                        .ok();
                }
                Event::NodeStart(NodeStart {
                    time,
                    node_id,
                    times,
                    input,
                }) => {
                    conn.new_node_run(&run_id, &node_id, &(times as i32), &time, &strip(input))
                        .await
                        .map_err(log_error)
                        .ok();
                }
                Event::NodeOutput(NodeOutput {
                    node_id,
                    times,
                    output,
                    ..
                }) => {
                    conn.save_node_output(&run_id, &node_id, &(times as i32), &strip(output))
                        .await
                        .map_err(log_error)
                        .ok();
                }
                Event::NodeError(NodeError {
                    node_id,
                    times,
                    error,
                    ..
                }) => {
                    conn.push_node_error(&run_id, &node_id, &(times as i32), &error)
                        .await
                        .map_err(log_error)
                        .ok();
                }
                Event::NodeFinish(NodeFinish {
                    time,
                    node_id,
                    times,
                }) => {
                    conn.set_node_finish(&run_id, &node_id, &(times as i32), &time)
                        .await
                        .map_err(log_error)
                        .ok();
                }
                Event::NodeLog(NodeLog {
                    time,
                    node_id,
                    times,
                    level,
                    module,
                    content,
                }) => {
                    logs.push(FlowRunLogsRow {
                        user_id,
                        flow_run_id: run_id,
                        log_index,
                        node_id: Some(node_id),
                        times: Some(times as i32),
                        time,
                        log_level: level.to_string(),
                        content,
                        module,
                    });
                    log_index += 1;
                }
                Event::FlowLog(FlowLog {
                    time,
                    level,
                    module,
                    content,
                }) => {
                    logs.push(FlowRunLogsRow {
                        user_id,
                        flow_run_id: run_id,
                        log_index,
                        node_id: None,
                        times: None,
                        time,
                        log_level: level.to_string(),
                        content,
                        module,
                    });
                    log_index += 1;
                }
            }
        }
        if !logs.is_empty() && tx.send(CopyIn(logs)).await.is_err() {
            tracing::error!("failed to send to DBWorker, dropping event.")
        }
    }
}
