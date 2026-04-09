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
    config::client::ClientConfig,
    flow_run_events::{
        self, ApiInput, Event, FlowError, FlowFinish, FlowLog, NodeError, NodeFinish, NodeLog,
        NodeOutput, NodeStart,
    },
};
use futures_channel::mpsc;
use futures_util::{FutureExt, StreamExt, stream::BoxStream};
use metrics::histogram;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};
use thiserror::Error as ThisError;
use tokio::sync::broadcast::{self, error::RecvError};
use utils::address_book::ManagableActor;
use value::Value;

static ACTIVE_FLOW_RUNS: AtomicU64 = AtomicU64::new(0);
static ACTIVE_FLOW_RUN_SUBSCRIBERS: AtomicU64 = AtomicU64::new(0);
static BUFFERED_FLOW_RUN_EVENTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub(crate) struct VaultPlaceholderConfig {
    by_node: HashMap<flow_lib::NodeId, HashMap<String, String>>,
}

impl VaultPlaceholderConfig {
    pub(crate) fn from_client_config(config: &ClientConfig) -> Self {
        let mut by_node = HashMap::new();

        for node in &config.nodes {
            let Some(node_config) = node.data.config.as_object() else {
                continue;
            };
            let mut placeholders = HashMap::new();
            for (input_name, value) in node_config {
                let Some(vault_ref) = vault_ref_from_json(value) else {
                    continue;
                };
                placeholders.insert(input_name.clone(), format!("<vault:{vault_ref}>"));
            }
            if !placeholders.is_empty() {
                by_node.insert(node.id, placeholders);
            }
        }

        Self { by_node }
    }
}

#[derive(Debug, Default)]
struct SecretRedactor {
    secrets: HashMap<String, String>,
}

impl SecretRedactor {
    fn register_node_input(
        &mut self,
        node_id: flow_lib::NodeId,
        input: &Value,
        placeholders: &VaultPlaceholderConfig,
    ) {
        let Some(node_placeholders) = placeholders.by_node.get(&node_id) else {
            return;
        };
        let Value::Map(input_map) = input else {
            return;
        };
        for (input_name, placeholder) in node_placeholders {
            let Some(secret) = input_map.get(input_name).and_then(value_as_string) else {
                continue;
            };
            if !secret.is_empty() {
                self.secrets.insert(secret.to_owned(), placeholder.clone());
            }
        }
    }

    fn redact_string(&self, value: String) -> String {
        let mut redacted = value;
        let mut replacements = self
            .secrets
            .iter()
            .map(|(secret, placeholder)| (secret.as_str(), placeholder.as_str()))
            .collect::<Vec<_>>();
        replacements.sort_by_key(|(secret, _)| std::cmp::Reverse(secret.len()));
        for (secret, placeholder) in replacements {
            if !secret.is_empty() && redacted.contains(secret) {
                redacted = redacted.replace(secret, placeholder);
            }
        }
        redacted
    }

    fn redact_value(&self, value: Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.redact_string(s)),
            Value::Array(values) => Value::Array(
                values
                    .into_iter()
                    .map(|value| self.redact_value(value))
                    .collect(),
            ),
            Value::Map(values) => Value::Map(
                values
                    .into_iter()
                    .map(|(key, value)| (key, self.redact_value(value)))
                    .collect(),
            ),
            other => other,
        }
    }

    fn redact_event(&mut self, event: Event, placeholders: &VaultPlaceholderConfig) -> Event {
        match event {
            Event::NodeStart(mut event) => {
                self.register_node_input(event.node_id, &event.input, placeholders);
                event.input = self.redact_value(event.input);
                Event::NodeStart(event)
            }
            Event::NodeOutput(mut event) => {
                event.output = self.redact_value(event.output);
                Event::NodeOutput(event)
            }
            Event::NodeError(mut event) => {
                event.error = self.redact_string(event.error);
                Event::NodeError(event)
            }
            Event::NodeLog(mut event) => {
                event.content = self.redact_string(event.content);
                Event::NodeLog(event)
            }
            Event::FlowError(mut event) => {
                event.error = self.redact_string(event.error);
                Event::FlowError(event)
            }
            Event::FlowLog(mut event) => {
                event.content = self.redact_string(event.content);
                Event::FlowLog(event)
            }
            Event::FlowFinish(mut event) => {
                event.output = self.redact_value(event.output);
                Event::FlowFinish(event)
            }
            other => other,
        }
    }
}

fn vault_ref_from_json(value: &serde_json::Value) -> Option<String> {
    value
        .as_object()?
        .get("vault_ref")?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn value_as_string(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn atomic_saturating_sub(atomic: &AtomicU64, value: u64) {
    let mut current = atomic.load(Ordering::Relaxed);
    loop {
        let next = current.saturating_sub(value);
        match atomic.compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

fn update_flow_run_gauges() {
    metrics::gauge!("flow_run_active").set(ACTIVE_FLOW_RUNS.load(Ordering::Relaxed) as f64);
    metrics::gauge!("flow_run_subscribers")
        .set(ACTIVE_FLOW_RUN_SUBSCRIBERS.load(Ordering::Relaxed) as f64);
    metrics::gauge!("flow_run_buffered_events")
        .set(BUFFERED_FLOW_RUN_EVENTS.load(Ordering::Relaxed) as f64);
}

fn event_type_label(event: &Event) -> &'static str {
    match event {
        Event::FlowStart(_) => "flow_start",
        Event::FlowError(_) => "flow_error",
        Event::FlowFinish(_) => "flow_finish",
        Event::FlowLog(_) => "flow_log",
        Event::NodeStart(_) => "node_start",
        Event::NodeOutput(_) => "node_output",
        Event::NodeError(_) => "node_error",
        Event::NodeFinish(_) => "node_finish",
        Event::NodeLog(_) => "node_log",
        Event::SignatureRequest(_) => "signature_request",
        Event::ApiInput(_) => "api_input",
    }
}

pub struct FlowRunWorker {
    user_id: UserId,
    shared_with: Vec<UserId>,
    run_id: FlowRunId,
    vault_placeholders: VaultPlaceholderConfig,
    secret_redactor: SecretRedactor,
    stop_signal: StopSignal,
    stop_shared_signal: StopSignal,
    counter: Counter,
    tx: mpsc::UnboundedSender<Event>,
    subs: HashMap<SubscriptionID, Subscription>,
    all_events: Vec<Event>,
    done_tx: broadcast::Sender<()>,
    finished: bool,
}

impl Actor for FlowRunWorker {
    type Context = actix::Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        ACTIVE_FLOW_RUNS.fetch_add(1, Ordering::Relaxed);
        update_flow_run_gauges();
        tracing::debug!("started FlowRunWorker {}", self.run_id);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        atomic_saturating_sub(&ACTIVE_FLOW_RUNS, 1);
        atomic_saturating_sub(&ACTIVE_FLOW_RUN_SUBSCRIBERS, self.subs.len() as u64);
        atomic_saturating_sub(&BUFFERED_FLOW_RUN_EVENTS, self.all_events.len() as u64);
        update_flow_run_gauges();
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
        if self.finished {
            return async { Ok(()) }.boxed();
        }
        let mut rx = self.done_tx.subscribe();
        async move { rx.recv().await }.boxed()
    }
}

pub struct ForceStopFlow {
    pub timeout_millies: u32,
    pub reason: Option<String>,
}

impl actix::Message for ForceStopFlow {
    type Result = ();
}

impl actix::Handler<ForceStopFlow> for FlowRunWorker {
    type Result = ();

    fn handle(&mut self, msg: ForceStopFlow, _: &mut Self::Context) -> Self::Result {
        self.stop_signal.stop(msg.timeout_millies, msg.reason);
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
        metrics::counter!("flow_run_subscriptions_total").increment(1);
        metrics::counter!("flow_run_replayed_events_total").increment(self.all_events.len() as u64);
        histogram!("flow_run_subscribe_replay_events").record(self.all_events.len() as f64);
        for item in self.all_events.iter().cloned() {
            tx.unbounded_send(item).unwrap();
        }
        self.subs.insert(stream_id, Subscription { tx });
        ACTIVE_FLOW_RUN_SUBSCRIBERS.fetch_add(1, Ordering::Relaxed);
        update_flow_run_gauges();

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
    pub(crate) fn new(
        run_id: FlowRunId,
        user_id: UserId,
        shared_with: Vec<UserId>,
        vault_placeholders: VaultPlaceholderConfig,
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
            act.finished = true;
            act.done_tx.send(()).ok();
        }));
        ctx.add_stream(stream);

        FlowRunWorker {
            user_id,
            shared_with,
            run_id,
            vault_placeholders,
            secret_redactor: SecretRedactor::default(),
            stop_signal,
            stop_shared_signal,
            counter,
            tx,
            done_tx: broadcast::channel::<()>(1).0,
            subs: HashMap::new(),
            all_events: Vec::new(),
            finished: false,
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
        let item = self
            .secret_redactor
            .redact_event(item, &self.vault_placeholders);
        let is_finished = matches!(&item, Event::FlowFinish(_));
        metrics::counter!("flow_run_events_total", "type" => event_type_label(&item)).increment(1);

        self.tx.unbounded_send(item.clone()).ok();
        if is_finished {
            self.tx.close_channel();
        }

        let previous_sub_count = self.subs.len();
        self.subs.retain(|_, sub| {
            let retain = sub.tx.unbounded_send(item.clone()).is_ok() && !is_finished;
            if is_finished {
                sub.tx.close_channel();
            }
            retain
        });
        let removed = previous_sub_count.saturating_sub(self.subs.len());
        if removed > 0 {
            atomic_saturating_sub(&ACTIVE_FLOW_RUN_SUBSCRIBERS, removed as u64);
        }
        self.all_events.push(item);
        BUFFERED_FLOW_RUN_EVENTS.fetch_add(1, Ordering::Relaxed);
        update_flow_run_gauges();
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
    let mut finished = None;
    while let Some(events) = chunks.next().await {
        let chunk_started = Instant::now();
        tracing::trace!("events count: {}", events.len());
        histogram!("flow_run_save_to_db_chunk_events").record(events.len() as f64);
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
                            input: Some(input),
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
                    finished = Some(time);
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
        histogram!("flow_run_logs_batch_rows").record(logs.len() as f64);
        if !logs.is_empty() && tx.send(CopyIn(logs)).await.is_err() {
            metrics::counter!("flow_run_log_batches_dropped_total").increment(1);
            tracing::error!("failed to send to DBWorker, dropping events.")
        }
        histogram!("flow_run_save_to_db_chunk_seconds")
            .record(chunk_started.elapsed().as_secs_f64());
        if let Some(time) = finished {
            report(time, "flow_finish");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::{
        CommandType, FlowId, UserId,
        config::client::{ClientConfig, FlowRunOrigin, InputPort, Network, Node, NodeData},
    };
    use serde_json::json;
    use uuid::Uuid;

    fn sample_client_config() -> ClientConfig {
        ClientConfig {
            user_id: UserId::nil(),
            id: FlowId::nil(),
            nodes: vec![Node {
                id: Uuid::new_v4(),
                data: NodeData {
                    r#type: CommandType::Native,
                    node_id: "@spo/jupiter.price.0.1".to_owned(),
                    outputs: Vec::new(),
                    inputs: vec![InputPort {
                        id: Uuid::new_v4(),
                        name: "api_key".to_owned(),
                        type_bounds: Vec::new(),
                        required: false,
                        passthrough: false,
                        tooltip: None,
                    }],
                    config: json!({
                        "api_key": { "vault_ref": "jupiter/default" }
                    }),
                    wasm: None,
                    instruction_info: None,
                },
            }],
            edges: Vec::new(),
            environment: <_>::default(),
            sol_network: Network::default(),
            instructions_bundling: <_>::default(),
            partial_config: None,
            collect_instructions: false,
            call_depth: 0,
            origin: FlowRunOrigin::Start {},
            signers: serde_json::Value::Null,
            interflow_instruction_info: Err("not available".to_owned()),
        }
    }

    #[test]
    fn vault_placeholders_are_collected_from_saved_config() {
        let config = sample_client_config();
        let placeholders = VaultPlaceholderConfig::from_client_config(&config);
        let node_id = config.nodes[0].id;

        assert_eq!(
            placeholders
                .by_node
                .get(&node_id)
                .and_then(|node| node.get("api_key")),
            Some(&"<vault:jupiter/default>".to_owned())
        );
    }

    #[test]
    fn redactor_replaces_registered_secrets_recursively() {
        let config = sample_client_config();
        let placeholders = VaultPlaceholderConfig::from_client_config(&config);
        let node_id = config.nodes[0].id;
        let mut redactor = SecretRedactor::default();
        redactor.register_node_input(
            node_id,
            &Value::Map(
                [("api_key".to_owned(), Value::String("secret-123".to_owned()))]
                    .into_iter()
                    .collect(),
            ),
            &placeholders,
        );

        let output = Value::Map(
            [
                (
                    "url".to_owned(),
                    Value::String("https://api.test/?token=secret-123".to_owned()),
                ),
                (
                    "items".to_owned(),
                    Value::Array(vec![Value::String("secret-123".to_owned())]),
                ),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            redactor.redact_value(output),
            Value::Map(
                [
                    (
                        "url".to_owned(),
                        Value::String("https://api.test/?token=<vault:jupiter/default>".to_owned()),
                    ),
                    (
                        "items".to_owned(),
                        Value::Array(vec![Value::String("<vault:jupiter/default>".to_owned())]),
                    ),
                ]
                .into_iter()
                .collect(),
            )
        );
    }

    #[test]
    fn redactor_sanitizes_live_events_before_broadcast() {
        let config = sample_client_config();
        let placeholders = VaultPlaceholderConfig::from_client_config(&config);
        let node_id = config.nodes[0].id;
        let mut redactor = SecretRedactor::default();

        let node_start = redactor.redact_event(
            Event::NodeStart(NodeStart {
                time: Utc::now(),
                node_id,
                times: 0,
                input: Value::Map(
                    [("api_key".to_owned(), Value::String("secret-123".to_owned()))]
                        .into_iter()
                        .collect(),
                ),
            }),
            &placeholders,
        );
        let node_output = redactor.redact_event(
            Event::NodeOutput(NodeOutput {
                time: Utc::now(),
                node_id,
                times: 0,
                output: Value::String("token=secret-123".to_owned()),
            }),
            &placeholders,
        );

        match node_start {
            Event::NodeStart(NodeStart {
                node_id: id, input, ..
            }) => {
                assert_eq!(id, node_id);
                assert_eq!(
                    input,
                    Value::Map(
                        [(
                            "api_key".to_owned(),
                            Value::String("<vault:jupiter/default>".to_owned()),
                        )]
                        .into_iter()
                        .collect(),
                    )
                );
            }
            other => panic!("expected NodeStart, got {:?}", other),
        }
        match node_output {
            Event::NodeOutput(NodeOutput {
                node_id: id,
                output,
                ..
            }) => {
                assert_eq!(id, node_id);
                assert_eq!(
                    output,
                    Value::String("token=<vault:jupiter/default>".to_owned())
                );
            }
            other => panic!("expected NodeOutput, got {:?}", other),
        }
    }
}
