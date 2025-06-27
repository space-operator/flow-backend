use chrono::Utc;
use flow::flow_run_events::{Event, EventSender, FLOW_SPAN_NAME, FlowLog, NODE_SPAN_NAME, NodeLog};
use flow_lib::NodeId;
use hashbrown::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{Subscriber, span};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{EnvFilter, Layer, layer::Filter, registry::LookupSpan};

#[derive(Debug)]
pub struct Data {
    pub tx: EventSender,
    pub filter: EnvFilter,
}

pub type Map = Arc<RwLock<HashMap<tracing::span::Id, Data>>>;

pub struct FlowLogs {
    map: Map,
}

impl FlowLogs {
    pub fn new() -> (Self, Map) {
        let map = Map::default();
        (Self { map: map.clone() }, map)
    }
}

#[derive(Default, Clone)]
struct LogMessage {
    message: Option<String>,
}

impl tracing::field::Visit for LogMessage {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        }
    }
}

fn get_message(event: &tracing::Event<'_>) -> Option<String> {
    let mut msg = LogMessage::default();
    event.record(&mut msg);
    msg.message
}

#[derive(Clone)]
struct NodeLogSpan {
    node_id: NodeId,
    times: u32,
}

#[derive(Default, Clone, Debug)]
struct NodeLogSpanVisitor {
    node_id: Option<NodeId>,
    times: Option<u32>,
}

impl NodeLogSpanVisitor {
    fn finish(self) -> Option<NodeLogSpan> {
        Some(NodeLogSpan {
            node_id: self.node_id?,
            times: self.times?,
        })
    }
}

impl NodeLogSpanVisitor {
    fn record_times<T: TryInto<u32>>(&mut self, field: &tracing::field::Field, value: T) {
        if field.name() == "times" {
            if let Ok(u) = value.try_into() {
                self.times = Some(u);
            }
        }
    }
}

impl tracing::field::Visit for NodeLogSpanVisitor {
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
            if let Ok(id) = value.parse::<NodeId>() {
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
        self.record_str(field, &format!("{value:?}"));
    }
}

impl<S> Layer<S> for FlowLogs
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        cx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if attrs.metadata().name() == NODE_SPAN_NAME {
            let span = match cx.span(id) {
                None => return,
                Some(span) => span,
            };
            let mut ext = span.extensions_mut();
            if ext.get_mut::<NodeLogSpan>().is_none() {
                let mut fields = NodeLogSpanVisitor::default();
                attrs.record(&mut fields);
                if let Some(value) = fields.finish() {
                    ext.insert(value);
                }
            }
        }
    }

    fn on_close(&self, id: span::Id, cx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = cx.span(&id) {
            if span.name() == FLOW_SPAN_NAME {
                self.map.write().unwrap().remove(&id);
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, cx: tracing_subscriber::layer::Context<'_, S>) {
        let map = self.map.read().unwrap();
        let result = cx
            .event_scope(event)
            .and_then(|mut iter| iter.find_map(|span| map.get_key_value(&span.id())));
        if let Some((id, data)) = result {
            let id = id.clone();
            if Filter::enabled(&data.filter, event.metadata(), &cx) {
                let content = match get_message(event) {
                    Some(s) => s,
                    None => return,
                };

                let normalized_metadata = event.normalized_metadata();
                let meta = normalized_metadata
                    .as_ref()
                    .unwrap_or_else(|| event.metadata());

                let level = *meta.level();
                let module = meta.module_path().map(<_>::to_owned);

                let node_log = cx.event_scope(event).and_then(|mut iter| {
                    iter.find_map(|span| span.extensions().get::<NodeLogSpan>().cloned())
                });

                let time = Utc::now();
                let log = match node_log {
                    None => Event::FlowLog(FlowLog {
                        time,
                        level: level.into(),
                        module,
                        content,
                    }),
                    Some(NodeLogSpan { node_id, times }) => Event::NodeLog(NodeLog {
                        time,
                        node_id,
                        times,
                        level: level.into(),
                        module,
                        content,
                    }),
                };

                if data.tx.unbounded_send(log).is_err() {
                    drop(map);

                    self.map.write().unwrap().remove(&id);
                }
            }
        }
    }
}

pub struct IgnoreFlowLogs {
    map: Map,
}

impl IgnoreFlowLogs {
    pub fn new(map: Map) -> Self {
        Self { map }
    }
}

impl<S> Filter<S> for IgnoreFlowLogs
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(
        &self,
        meta: &tracing::Metadata<'_>,
        cx: &tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        if meta.is_span() {
            true
        } else {
            let map = self.map.read().unwrap();
            let data = cx.lookup_current().and_then(|span| {
                cx.span_scope(&span.id())
                    .and_then(|mut iter| iter.find_map(|span| map.get(&span.id())))
            });
            data.is_none()
        }
    }
}
