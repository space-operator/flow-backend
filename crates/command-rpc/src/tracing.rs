use ahash::AHashMap;
use bincode::config::standard;
use flow_lib::{
    FlowRunId, NodeId,
    flow_run_events::{
        self, Event, EventReceiver, EventSender, NODE_SPAN_NAME, NodeLog, NodeLogContent,
        NodeLogSender,
    },
};
use flow_tracing::FlowLogs;
use futures::StreamExt;
use std::{cell::RefCell, collections::hash_map::Entry, rc::Rc, sync::OnceLock};
use tokio::task::spawn_local;
use tracing::Span;
use tracing_subscriber::prelude::*;

use crate::flow_side::command_context;

pub struct Tracker {
    span: Span,
    clients: Rc<RefCell<AHashMap<(NodeId, u32), command_context::Client>>>,
    tx: EventSender,
}

async fn send_log(
    client: &command_context::Client,
    log: NodeLogContent,
) -> Result<(), anyhow::Error> {
    let mut req = client.log_request();
    let data = bincode::encode_to_vec(&log, standard())?;
    req.get().set_log(&data);
    req.send().promise.await?;
    Ok(())
}

type ClientsMap = Rc<RefCell<AHashMap<(NodeId, u32), command_context::Client>>>;

async fn drive(mut rx: EventReceiver, clients: ClientsMap, run_id: FlowRunId) {
    while let Some(Event::NodeLog(NodeLog {
        time,
        node_id,
        times,
        level,
        module,
        content,
    })) = rx.next().await
    {
        let client = clients.borrow().get(&(node_id, times)).cloned();
        if let Some(client) = client {
            if let Err(error) = send_log(
                &client,
                NodeLogContent {
                    time,
                    level,
                    module,
                    content,
                },
            )
            .await
            {
                tracing::error!("send_log error: {}", error);
            }
        } else {
            tracing::error!("no client registered {}:{}:{}", run_id, node_id, times)
        }
    }
}

fn init_tracing_flow_logs() -> FlowLogs {
    let (logs, ignore) = flow_tracing::new();
    let env = tracing_subscriber::EnvFilter::builder().parse_lossy(
        std::env::var("RUST_LOG")
            .as_deref()
            .unwrap_or("info,iroh=error"),
    );
    let fmt = tracing_subscriber::fmt::layer()
        .with_filter(env)
        .with_filter(ignore);
    tracing_subscriber::registry()
        .with(fmt)
        .with(logs.clone())
        .init();
    logs
}

#[derive(Clone)]
pub struct TrackFlowRun {
    flow_logs: FlowLogs,
    runs: Rc<RefCell<AHashMap<FlowRunId, Tracker>>>,
}

impl TrackFlowRun {
    pub fn init_tracing_once() -> Self {
        static LOGS: OnceLock<FlowLogs> = OnceLock::new();

        let logs = LOGS.get_or_init(init_tracing_flow_logs).clone();

        Self::new(logs)
    }

    pub fn init_tracing() -> Self {
        Self::new(init_tracing_flow_logs())
    }

    pub fn new(flow_logs: FlowLogs) -> Self {
        Self {
            flow_logs,
            runs: <_>::default(),
        }
    }

    pub fn enter(
        &self,
        run_id: FlowRunId,
        filter: &str,
        node_id: NodeId,
        times: u32,
        client: command_context::Client,
    ) -> (Span, NodeLogSender) {
        tracing::debug!("tracker enter: {run_id}:{node_id}:{times}");
        let mut runs = self.runs.borrow_mut();
        let tracker = runs.entry(run_id).or_insert_with(|| {
            let (tx, rx) = flow_run_events::channel();
            let clients = ClientsMap::default();
            spawn_local(drive(rx, clients.clone(), run_id));
            let span = self
                .flow_logs
                .register_flow_logs(run_id, filter, tx.clone());
            Tracker { span, clients, tx }
        });
        tracker
            .clients
            .borrow_mut()
            .insert((node_id, times), client);
        let flow_span = tracker.span.clone();
        let node_span = tracing::error_span!(parent: flow_span, NODE_SPAN_NAME, node_id = node_id.to_string(), times = times);
        let sender = NodeLogSender::new(tracker.tx.clone(), node_id, times);
        (node_span, sender)
    }

    pub fn exit(&self, run_id: FlowRunId, node_id: NodeId, times: u32) {
        tracing::debug!("tracker exit: {run_id}:{node_id}:{times}");
        let mut runs = self.runs.borrow_mut();
        if let Entry::Occupied(mut tracker) = runs.entry(run_id) {
            let delete = {
                let mut clients = tracker.get_mut().clients.borrow_mut();
                clients.remove(&(node_id, times));
                clients.is_empty()
            };
            if delete {
                tracker.remove_entry();
            }
        }
    }
}
