use anyhow::anyhow;
use chrono::{DateTime, Utc};
use flow_lib::NodeId;
use serde::Serialize;
use tracing::{span, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    filter::LevelFilter, prelude::__tracing_subscriber_SubscriberExt, registry::LookupSpan,
    EnvFilter, Layer,
};
use uuid::Uuid;
use value::Value;

#[derive(derive_more::From, actix::Message, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub enum Event {
    FlowStart(FlowStart),
    FlowError(FlowError),
    FlowLog(FlowLog),
    FlowFinish(FlowFinish),
    NodeStart(NodeStart),
    NodeOutput(NodeOutput),
    NodeError(NodeError),
    NodeLog(NodeLog),
    NodeFinish(NodeFinish),
}

impl Event {
    pub fn time(&self) -> DateTime<Utc> {
        match self {
            Event::FlowStart(e) => e.time,
            Event::FlowError(e) => e.time,
            Event::FlowLog(e) => e.time,
            Event::FlowFinish(e) => e.time,
            Event::NodeStart(e) => e.time,
            Event::NodeOutput(e) => e.time,
            Event::NodeError(e) => e.time,
            Event::NodeLog(e) => e.time,
            Event::NodeFinish(e) => e.time,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

impl From<tracing::Level> for LogLevel {
    fn from(value: tracing::Level) -> Self {
        match value {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowStart {
    #[serde(skip)]
    pub time: DateTime<Utc>,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowError {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub error: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowLog {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub level: LogLevel,
    pub module: Option<String>,
    pub content: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowFinish {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub not_run: Vec<NodeId>,
    pub output: Value,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeStart {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub input: Value,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeOutput {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub output: Value,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeError {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub error: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeLog {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub level: LogLevel,
    pub module: Option<String>,
    pub content: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeFinish {
    #[serde(skip)]
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
}

pub type EventSender = futures::channel::mpsc::UnboundedSender<Event>;
pub type EventReceiver = futures::channel::mpsc::UnboundedReceiver<Event>;

pub fn event_channel() -> (EventSender, EventReceiver) {
    futures::channel::mpsc::unbounded()
}

pub struct TracingLayer {
    tx: EventSender,
}

impl TracingLayer {
    pub fn new(tx: EventSender) -> Self {
        Self { tx }
    }
}

#[derive(Default, Clone, Debug)]
struct Fields {
    node_id: Option<Uuid>,
    times: Option<u32>,
}

impl Fields {
    fn record_times<T: TryInto<u32>>(&mut self, field: &tracing::field::Field, value: T) {
        if field.name() == "times" {
            if let Ok(u) = value.try_into() {
                self.times = Some(u);
            }
        }
    }
}

impl tracing::field::Visit for Fields {
    fn record_f64(&mut self, _field: &tracing::field::Field, _value: f64) {}

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.record_times(field, value);
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.record_times(field, value);
    }

    fn record_i128(&mut self, field: &tracing::field::Field, value: i128) {
        self.record_times(field, value);
    }

    fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
        self.record_times(field, value);
    }

    fn record_bool(&mut self, _field: &tracing::field::Field, _value: bool) {}

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "node_id" {
            if let Ok(id) = value.parse::<Uuid>() {
                self.node_id = Some(id);
            }
        }
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.record_str(field, &value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.record_str(field, &format!("{:?}", value));
    }
}

#[derive(Default, Clone)]
struct LogMessage {
    message: Option<String>,
}

impl tracing::field::Visit for LogMessage {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.record_str(field, &format!("{:?}", value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.record_str(field, &value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.record_str(field, &value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.record_str(field, &value.to_string());
    }

    fn record_i128(&mut self, field: &tracing::field::Field, value: i128) {
        self.record_str(field, &value.to_string());
    }

    fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
        self.record_str(field, &value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.record_str(field, &value.to_string());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        }
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.record_str(field, &value.to_string());
    }
}

impl<S> Layer<S> for TracingLayer
where
    S: Subscriber,
    S: for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let s = ctx.span(id).unwrap();
        let mut ext = s.extensions_mut();
        if ext.get_mut::<Fields>().is_none() {
            let mut fields = Fields::default();
            attrs.record(&mut fields);
            ext.insert(fields);
        }
    }

    fn on_record(
        &self,
        id: &span::Id,
        values: &span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let s = ctx.span(id).unwrap();
        let mut ext = s.extensions_mut();
        let fields = ext.get_mut::<Fields>().unwrap();
        values.record(fields);
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Err(error) = self.on_event_fallible(event, ctx) {
            // NOTE: beware of infinite loop
            tracing::error!("error processing log: {}", error);
        }
    }
}

impl TracingLayer {
    fn on_event_fallible<S>(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> Result<(), anyhow::Error>
    where
        S: Subscriber,
        S: for<'a> LookupSpan<'a>,
    {
        let time = Utc::now();
        let span_id = event
            .parent()
            .cloned()
            .or_else(|| ctx.current_span().id().cloned());
        let fields = match span_id {
            Some(id) => {
                let span = ctx.span(&id).ok_or_else(|| anyhow!("span not found"))?;
                let ext = span.extensions();
                let fields = ext
                    .get::<Fields>()
                    .ok_or_else(|| anyhow!("span not registered"))?;
                Some(fields.clone())
            }
            None => None,
        };

        let normalized_metadata = event.normalized_metadata();
        let meta = normalized_metadata
            .as_ref()
            .unwrap_or_else(|| event.metadata());

        let level = *meta.level();
        let module = meta.module_path().map(<_>::to_owned);

        let content = {
            let mut msg = LogMessage::default();
            event.record(&mut msg);
            match msg.message {
                Some(s) => s,
                None => return Ok(()),
            }
        };

        let event = match fields {
            Some(Fields {
                node_id: Some(node_id),
                times: Some(times),
            }) => Event::NodeLog(NodeLog {
                time,
                node_id,
                times,
                level: level.into(),
                module,
                content,
            }),
            _ => Event::FlowLog(FlowLog {
                time,
                level: level.into(),
                module,
                content,
            }),
        };
        self.tx.unbounded_send(event)?;
        Ok(())
    }
}

pub const DEFAULT_LOG_FILTER: &str = "info,solana_client=debug";

pub fn build_tracing_subscriber(
    tx: EventSender,
    filter: Option<&str>,
) -> impl Into<tracing::Dispatch> {
    tracing_subscriber::registry().with(
        TracingLayer::new(tx.clone()).with_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::ERROR.into())
                .parse_lossy(filter.unwrap_or(DEFAULT_LOG_FILTER)),
        ),
    )
}
