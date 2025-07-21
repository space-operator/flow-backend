use std::{cell::RefCell, collections::hash_map::Entry, rc::Rc};

use ahash::AHashMap;
use bincode::config::standard;
use flow_lib::{
    FlowRunId, NodeId,
    flow_run_events::{
        self, Event, EventReceiver, EventSender, NodeLog, NodeLogContent, NodeLogSender,
    },
};
use flow_tracing::FlowLogs;
use futures::StreamExt;
use tokio::task::spawn_local;
use tracing::Span;

use crate::flow_side::command_context;

pub struct Tracker {
    span: Span,
    clients: Rc<RefCell<AHashMap<(NodeId, u32), command_context::Client>>>,
    tx: EventSender,
}

#[derive(Clone)]
pub struct TrackFlowRun {
    flow_logs: FlowLogs,
    runs: Rc<RefCell<AHashMap<FlowRunId, Tracker>>>,
}

async fn send_log(
    client: &command_context::Client,
    log: NodeLogContent,
) -> Result<(), anyhow::Error> {
    let mut req = client.log_request();
    let data = bincode::encode_to_vec(&log, standard())?;
    req.get().set_log(&data);
    Ok(())
}

type ClientsMap = Rc<RefCell<AHashMap<(NodeId, u32), command_context::Client>>>;

async fn drive(mut rx: EventReceiver, clients: ClientsMap) {
    while let Some(Event::NodeLog(NodeLog {
        time,
        node_id,
        times,
        level,
        module,
        content,
    })) = rx.next().await
    {
        if let Some(client) = clients.borrow_mut().get(&(node_id, times)) {
            if let Err(error) = send_log(
                client,
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
        }
    }
}

impl TrackFlowRun {
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
        let mut runs = self.runs.borrow_mut();
        let tracker = runs.entry(run_id).or_insert_with(|| {
            let (tx, rx) = flow_run_events::channel();
            let clients = ClientsMap::default();
            spawn_local(drive(rx, clients.clone()));
            let span = self
                .flow_logs
                .register_flow_logs(run_id, filter, tx.clone());
            Tracker { span, clients, tx }
        });
        tracker
            .clients
            .borrow_mut()
            .insert((node_id, times), client);
        (
            tracker.span.clone(),
            NodeLogSender::new(tracker.tx.clone(), node_id, times),
        )
    }

    pub fn exit(&self, run_id: FlowRunId, node_id: NodeId, times: u32) {
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
