use super::{
    CopyIn, Counter, DBWorker, SystemShutdown,
    messages::{SubscribeError, SubscriptionID},
    user_worker::SigReqEvent,
};
use crate::{
    api::prelude::AuthEither,
    error::ErrorBody,
    middleware::auth_v1::{AuthenticatedUser, FlowRunToken},
};
use actix::{
    Actor, ActorContext, ActorFutureExt, AsyncContext, ResponseActFuture, ResponseFuture,
    StreamHandler, WrapFuture, fut::wrap_future,
};
use actix_web::http::StatusCode;
use ahash::{HashMap, HashMapExt};
use chrono::{DateTime, Utc};
use db::{FlowRunLogsRow, connection::PartialNodeRunRow, pool::DbPool};
use flow::flow_graph::StopSignal;
use flow_lib::{
    FlowRunId, UserId,
    flow_run_events::{
        self, ApiInput, Event, FlowError, FlowFinish, FlowLog, NodeError, NodeFinish, NodeLog,
        NodeOutput, NodeStart,
    },
};
use futures_channel::mpsc;
use futures_util::{FutureExt, StreamExt, stream::BoxStream};
use metrics::{Unit, describe_histogram, histogram};
use thiserror::Error as ThisError;
use tokio::sync::broadcast::{self, error::RecvError};
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
        tracing::debug!("started FlowRunWorker {}", self.run_id);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::debug!("stopped FlowRunWorker {}", self.run_id);
        self.stop_signal
            .stop(0, Some("stopping FlowRunWorker".to_owned()));
    }
}

impl ManagableActor for FlowRunWorker {
    type ID = FlowRunId;

    fn id(&self) -> Self::ID {
        self.run_id
    }
}

pub struct WaitFinish;

impl actix::Message for WaitFinish {
    type Result = Result<(), RecvError>;
}

impl actix::Handler<WaitFinish> for FlowRunWorker {
    type Result = ResponseFuture<<WaitFinish as actix::Message>::Result>;
    fn handle(&mut self, _: WaitFinish, _: &mut Self::Context) -> Self::Result {
        let mut rx = self.done_tx.subscribe();
        async move { rx.recv().await }.boxed()
    }
}

impl actix::Handler<ApiInput> for FlowRunWorker {
    type Result = ();
    fn handle(&mut self, msg: ApiInput, ctx: &mut Self::Context) -> Self::Result {
        StreamHandler::handle(self, Event::ApiInput(msg), ctx)
    }
}

impl actix::Handler<SigReqEvent> for FlowRunWorker {
    type Result = ();
    fn handle(&mut self, msg: SigReqEvent, ctx: &mut Self::Context) -> Self::Result {
        StreamHandler::handle(self, Event::SignatureRequest(msg.0), ctx)
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
                    stop_signal.stop(0, Some("restarting server".to_owned()));
                    rx.recv().await.ok();
                }
            }
            .into_actor(&*self)
            .map(|_, _, ctx| ctx.stop()),
        )
    }
}

struct Subscription {
    tx: mpsc::UnboundedSender<Event>,
}

pub struct SubscribeEvents {
    pub tokens: Vec<AuthEither<AuthenticatedUser, FlowRunToken>>,
}

impl actix::Message for SubscribeEvents {
    type Result = Result<(SubscriptionID, mpsc::UnboundedReceiver<Event>), SubscribeError>;
}

impl actix::Handler<SubscribeEvents> for FlowRunWorker {
    type Result = <SubscribeEvents as actix::Message>::Result;

    fn handle(&mut self, msg: SubscribeEvents, _: &mut Self::Context) -> Self::Result {
        let can_read = msg
            .tokens
            .iter()
            .any(|token| token.is_user(&self.user_id) || token.is_flow_run(&self.run_id));
        if !can_read {
            return Err(SubscribeError::Unauthorized);
        }

        let stream_id = self.counter.next();
        let (tx, rx) = mpsc::unbounded();
        for item in self.all_events.iter().cloned() {
            tx.unbounded_send(item).unwrap();
        }
        self.subs.insert(stream_id, Subscription { tx });

        Ok((stream_id, rx))
    }
}

pub struct StopFlow {
    pub user_id: UserId,
    pub run_id: FlowRunId,
    pub timeout_millies: u32,
    pub reason: Option<String>,
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
                self.stop_shared_signal
                    .stop(msg.timeout_millies, msg.reason);
                return Ok(());
            }
            return Err(StopError::Unauthorized {
                user_id: msg.user_id,
            });
        }
        if self.run_id != msg.run_id {
            return Err(StopError::NotFound);
        }
        self.stop_signal.stop(msg.timeout_millies, msg.reason);
        Ok(())
    }
}

impl FlowRunWorker {
    #[allow(clippy::too_many_arguments)]
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

        self.subs.retain(|_, sub| {
            let retain = sub.tx.unbounded_send(item.clone()).is_ok() && !is_finished;
            if is_finished {
                sub.tx.close_channel();
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

fn report(time: DateTime<Utc>, ty: &'static str) {
    let lag = Utc::now() - time;
    metrics::histogram!("event_lag", "type" => ty).record(lag.as_seconds_f64());
}

async fn save_to_db(
    user_id: UserId,
    run_id: FlowRunId,
    rx: mpsc::UnboundedReceiver<Event>,
    db: DbPool,
    tx: actix::Recipient<CopyIn<Vec<FlowRunLogsRow>>>,
) {
    let mut log_index = 0i32;
    const CHUNK_SIZE: usize = 64;
    let mut chunks = rx.ready_chunks(CHUNK_SIZE);
    let mut stop = false;
    let mut flow_finish_time = None;
    while let Some(events) = chunks.next().await {
        tracing::trace!("events count: {}", events.len());
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

        let mut before = Vec::new();
        let mut new_nodes = HashMap::new();
        let mut after = Vec::new();
        for event in events {
            match event {
                Event::NodeStart(NodeStart {
                    time,
                    node_id,
                    times,
                    input,
                }) => {
                    new_nodes.insert(
                        (node_id, times),
                        PartialNodeRunRow {
                            user_id,
                            flow_run_id: run_id,
                            node_id,
                            times,
                            start_time: Some(time),
                            end_time: None,
                            input: Some(input.clone()),
                            output: None,
                            errors: None,
                        },
                    );
                }
                Event::NodeOutput(NodeOutput {
                    node_id,
                    times,
                    output,
                    time,
                }) => match new_nodes.get_mut(&(node_id, times)) {
                    Some(node) => {
                        node.output = Some(output);
                    }
                    None => {
                        after.push(Event::NodeOutput(NodeOutput {
                            node_id,
                            times,
                            output,
                            time,
                        }));
                    }
                },
                Event::NodeError(NodeError {
                    node_id,
                    times,
                    error,
                    time,
                }) => match new_nodes.get_mut(&(node_id, times)) {
                    Some(node) => {
                        if let Some(errors) = &mut node.errors {
                            errors.push(error.clone());
                        } else {
                            node.errors = Some([error.clone()].into());
                        }
                    }
                    None => {
                        after.push(Event::NodeError(NodeError {
                            node_id,
                            times,
                            error,
                            time,
                        }));
                    }
                },
                Event::NodeFinish(NodeFinish {
                    time,
                    node_id,
                    times,
                }) => match new_nodes.get_mut(&(node_id, times)) {
                    Some(node) => {
                        node.end_time = Some(time);
                    }
                    None => {
                        after.push(event);
                    }
                },
                Event::FlowStart(flow_start) => {
                    before.push(flow_start);
                }
                Event::FlowError(_) | Event::FlowFinish(_) => {
                    after.push(event);
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
                Event::SignatureRequest(_) => {}
                Event::ApiInput(_) => {}
            }
        }

        histogram!("batch_nodes_insert_size").record(new_nodes.len() as f64);
        histogram!("after_insert_size").record(after.len() as f64);

        for event in before {
            conn.set_start_time(&run_id, &event.time)
                .await
                .map_err(log_error)
                .ok();
        }

        conn.copy_in_node_run(new_nodes.into_values().collect())
            .await
            .map_err(log_error)
            .ok();

        for event in after {
            match event {
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
                    // FlowFinish is the final message
                    stop = true;
                    flow_finish_time = Some(time);
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
                Event::FlowStart(_)
                | Event::NodeStart(_)
                | Event::NodeLog(_)
                | Event::FlowLog(_)
                | Event::SignatureRequest(_)
                | Event::ApiInput(_) => {
                    unreachable!();
                }
            }
        }
        drop(conn);
        if !logs.is_empty() && tx.send(CopyIn(logs)).await.is_err() {
            tracing::error!("failed to send to DBWorker, dropping events.")
        }

        if stop {
            if let Some(time) = flow_finish_time {
                report(time, "flow_finish");
            }
            break;
        }
    }
}
