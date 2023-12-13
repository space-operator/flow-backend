use super::{
    messages::{Finished, SubscribeError, SubscriptionID},
    CopyIn, Counter, DBWorker,
};
use crate::error::ErrorBody;
use actix::{Actor, ActorContext, ActorFutureExt, AsyncContext, StreamHandler, WrapFuture};
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
use utils::address_book::ManagableActor;
use value::Value;

pub struct FlowRunWorker {
    root: actix::Addr<DBWorker>,
    user_id: UserId,
    shared_with: Vec<UserId>,
    run_id: FlowRunId,
    stop_signal: StopSignal,
    stop_shared_signal: StopSignal,
    counter: Counter,
    db: DbPool,
    stream: Option<BoxStream<'static, Event>>,
    tx: Option<mpsc::UnboundedSender<Event>>,
    subs: HashMap<SubscriptionID, Subscription>,
    all_events: Vec<Event>,
}

impl Actor for FlowRunWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("started FlowRunWorker {}", self.run_id);
        if let Some(stream) = self.stream.take() {
            let (tx, rx) = mpsc::unbounded();
            self.tx.replace(tx);
            ctx.spawn(
                save_to_db(
                    self.user_id,
                    self.run_id,
                    rx,
                    self.db.clone(),
                    self.root.clone().recipient(),
                )
                .into_actor(&*self)
                .map(|_, _, ctx| ctx.stop()),
            );
            ctx.add_stream(stream);
        } else {
            tracing::error!("started called twice");
        }
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

struct Subscription {
    finished: actix::WeakRecipient<Finished>,
    receiver: actix::WeakRecipient<FullEvent>,
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
}

impl actix::Message for SubscribeEvents {
    type Result = Result<(SubscriptionID, Vec<Event>), SubscribeError>;
}

impl actix::Handler<SubscribeEvents> for FlowRunWorker {
    type Result = Result<(SubscriptionID, Vec<Event>), SubscribeError>;

    fn handle(&mut self, msg: SubscribeEvents, _: &mut Self::Context) -> Self::Result {
        if msg.user_id != self.user_id && !self.shared_with.contains(&msg.user_id) {
            return Err(SubscribeError::Unauthorized);
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
    #[error("unauthorized")]
    Unauthorized,
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
            StopError::Unauthorized => StatusCode::UNAUTHORIZED,
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
            }
            return Err(StopError::Unauthorized);
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
    ) -> Self {
        FlowRunWorker {
            root,
            user_id,
            shared_with,
            run_id,
            stop_signal: StopSignal::new(),
            stop_shared_signal: StopSignal::new(),
            counter,
            db,
            stream: Some(stream),
            tx: None,
            subs: HashMap::new(),
            all_events: Vec::new(),
        }
    }

    pub fn stop_signal(&self) -> StopSignal {
        self.stop_signal.clone()
    }

    pub fn stop_shared_signal(&self) -> StopSignal {
        self.stop_signal.clone()
    }
}

impl StreamHandler<Event> for FlowRunWorker {
    fn handle(&mut self, item: Event, _: &mut Self::Context) {
        let tx = if let Some(tx) = &self.tx {
            tx
        } else {
            tracing::error!("stream received before `started`");
            return;
        };
        let is_finished = matches!(&item, Event::FlowFinish(_));

        tx.unbounded_send(item.clone()).ok();
        if is_finished {
            tx.close_channel();
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
        // TODO: is typed-arena faster?
        self.all_events.push(item);
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        if let Some(tx) = &self.tx {
            tx.close_channel();
        } else {
            ctx.stop();
        }
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
        let mut logs: Vec<FlowRunLogsRow> = Vec::with_capacity(CHUNK_SIZE);
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
            }
        }
        if !logs.is_empty() && tx.send(CopyIn(logs)).await.is_err() {
            tracing::error!("failed to send to DBWorker, dropping event.")
        }
    }
}
