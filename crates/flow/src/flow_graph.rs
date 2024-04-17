use crate::{
    command::flow_output::FLOW_OUTPUT,
    context::CommandFactory,
    flow_registry::FlowRegistry,
    flow_run_events::{
        EventSender, FlowError, FlowFinish, FlowStart, NodeError, NodeFinish, NodeOutput,
        NodeStart, NODE_SPAN_NAME,
    },
};
use chrono::{DateTime, Utc};
use flow_lib::{
    command::{CommandError, CommandTrait, InstructionInfo},
    config::client::{self, PartialConfig},
    context::{execute, get_jwt, CommandContext, Context},
    solana::{find_failed_instruction, ExecutionConfig, Instructions, KeypairExt},
    utils::{Extensions, TowerClient},
    CommandType, FlowConfig, FlowId, FlowRunId, Name, NodeId, ValueSet,
};
use futures::{
    channel::{mpsc, oneshot},
    future::{BoxFuture, Either},
    stream::FuturesUnordered,
    FutureExt, StreamExt,
};
use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;
use petgraph::{
    graph::EdgeIndex,
    stable_graph::{NodeIndex, StableGraph},
    visit::EdgeRef,
    Direction,
};
use solana_sdk::signature::Keypair;
use std::{
    collections::{BTreeSet, VecDeque},
    ops::ControlFlow,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, RwLock,
    },
    task::Poll,
    time::Duration,
};
use thiserror::Error as ThisError;
use tokio::{
    process::Child,
    sync::Semaphore,
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use uuid::Uuid;
use value::Value;

pub const MAX_STOP_TIMEOUT: u32 = Duration::from_secs(5 * 60).as_millis() as u32;

#[derive(Debug, Clone)]
pub struct StopSignal {
    pub token: CancellationToken,
    pub timeout_millies: Arc<AtomicU32>,
    pub reason: Arc<RwLock<Option<String>>>,
}

impl Default for StopSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl StopSignal {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            timeout_millies: Arc::new(AtomicU32::new(0)),
            reason: Arc::new(RwLock::new(None)),
        }
    }

    pub fn stop(&self, timeout_millies: u32, reason: Option<String>) {
        if !self.token.is_cancelled() {
            let timeout = timeout_millies.min(MAX_STOP_TIMEOUT);
            self.timeout_millies.store(timeout, Ordering::Relaxed);
            *self.reason.write().unwrap() = reason;
            self.token.cancel();
        }
    }

    pub fn get_reason(&self) -> Option<String> {
        self.reason.read().unwrap().clone()
    }

    pub async fn race<F, O, E, FE>(&self, task: F, canceled_error: FE) -> Result<O, E>
    where
        FE: FnOnce(Option<String>) -> E,
        F: std::future::Future<Output = Result<O, E>> + Unpin,
    {
        match futures::future::select(task, std::pin::pin!(self.token.cancelled())).await {
            Either::Left((result, _)) => result,
            Either::Right((_, task)) => {
                let timeout = self.timeout_millies.load(Ordering::Relaxed);
                if timeout == 0 {
                    Err(canceled_error(self.get_reason()))
                } else {
                    let duration = Duration::from_millis(timeout as u64);
                    match tokio::time::timeout(duration, task).await {
                        Ok(result) => result,
                        Err(_) => Err(canceled_error(self.get_reason())),
                    }
                }
            }
        }
    }
}

pub struct FlowGraph {
    pub id: FlowId,
    pub ctx: Context,
    pub g: StableGraph<NodeId, Edge>,
    pub nodes: HashMap<NodeId, Node>,
    pub mode: client::BundlingMode,
    pub output_instructions: bool,
    pub rhai_permit: Arc<Semaphore>,
    pub tx_exec_config: ExecutionConfig,
    pub spawned: Vec<Child>,
}

pub struct UsePreviousValue {
    node_id: NodeId,
    output_name: Name,
    foreach: bool,
}

pub struct Node {
    pub id: NodeId,
    pub command: Box<dyn CommandTrait>,
    pub form_inputs: ValueSet,
    /// Index in the graph
    pub idx: NodeIndex<u32>,
    /// List of input ports to use previous run's values
    pub use_previous_values: HashMap<Name, UsePreviousValue>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("command", &self.command.name())
            .field("form_inputs", &self.form_inputs)
            .field("idx", &self.idx)
            .finish()
    }
}

pub struct PartialOutput {
    pub node_id: NodeId,
    pub times: u32,
    pub output: Result<(Instructions, ValueSet), CommandError>,
    pub resp: oneshot::Sender<Result<execute::Response, execute::Error>>,
}

#[derive(Debug)]
pub struct Edge {
    pub from: Name,
    pub to: Name,
    pub values: VecDeque<EdgeValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrackEdgeValue {
    None,
    Element(NodeIndex<u32>, u32),
    Zip(BTreeSet<TrackEdgeValue>),
    Nest(Vec<TrackEdgeValue>),
}

impl TrackEdgeValue {
    fn is_array(&self) -> bool {
        !matches!(self, TrackEdgeValue::None)
    }
}

impl TrackEdgeValue {
    fn zip(&self, other: &Self) -> Self {
        if matches!(other, TrackEdgeValue::None) {
            return self.clone();
        }
        match self.clone() {
            TrackEdgeValue::None => other.clone(),
            TrackEdgeValue::Zip(mut set) => {
                match other {
                    TrackEdgeValue::Zip(other_set) => {
                        set.extend(other_set.clone());
                    }
                    other => {
                        set.insert(other.clone());
                    }
                }
                TrackEdgeValue::Zip(set)
            }
            this => {
                if this == *other {
                    this
                } else {
                    let mut set = BTreeSet::new();
                    set.insert(this);
                    set.insert(other.clone());
                    TrackEdgeValue::Zip(set)
                }
            }
        }
    }

    fn nest(&self, (nid, pos): (NodeIndex<u32>, u32)) -> Self {
        let other = TrackEdgeValue::Element(nid, pos);
        match self.clone() {
            TrackEdgeValue::None => other,
            TrackEdgeValue::Nest(mut vec) => {
                vec.push(other);
                TrackEdgeValue::Nest(vec)
            }
            this => TrackEdgeValue::Nest(vec![this, other]),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EdgeValue {
    value: Value,
    tracker: TrackEdgeValue,
}

#[derive(Debug)]
struct State {
    flow_run_id: FlowRunId,
    previous_values: HashMap<NodeId, Vec<Value>>,
    flow_inputs: value::Map,
    event_tx: EventSender,
    ran: HashMap<NodeId, u32>,
    running: FuturesUnordered<JoinHandle<Finished>>,
    running_info: HashMap<(NodeId, u32), RunningNodeInfo>,
    result: FlowRunResult,
    stop: StopSignal,
    stop_shared: StopSignal,
    out_tx: mpsc::UnboundedSender<PartialOutput>,
    out_rx: mpsc::UnboundedReceiver<PartialOutput>,
}

#[derive(Debug)]
struct RunningNodeInfo {
    id: NodeId,
    times: u32,
    node_idx: NodeIndex<u32>,
    command_name: Name,
    tracker: TrackEdgeValue,
    instruction_info: Option<InstructionInfo>,
    passthrough: value::Map,
    waiting: Option<Waiting>,
    instruction_sent: bool,
}

#[derive(Debug)]
struct Waiting {
    instructions: Instructions,
    resp: oneshot::Sender<Result<execute::Response, execute::Error>>,
}

fn node_error(
    event_tx: &EventSender,
    result: &mut FlowRunResult,
    node_id: NodeId,
    times: u32,
    error: String,
) {
    event_tx
        .unbounded_send(
            NodeError {
                time: Utc::now(),
                node_id,
                times,
                error: error.clone(),
            }
            .into(),
        )
        .ok();
    result
        .node_errors
        .entry((node_id, times))
        .or_default()
        .push(error);
}

impl State {
    fn flow_error(&mut self, error: String) {
        self.event_tx
            .unbounded_send(
                FlowError {
                    time: Utc::now(),
                    error: error.clone(),
                }
                .into(),
            )
            .ok();
        self.result.flow_errors.push(error);
    }

    async fn wait(mut self) -> (Self, Vec<PartialOutput>, Vec<Result<Finished, JoinError>>) {
        let len = self.running.len();
        let mut output_chunk = self.out_rx.ready_chunks(len);
        let mut node_chunk = self.running.ready_chunks(len);
        tracing::trace!("waiting for updates");
        let (outputs, finished) =
            match futures::future::select(output_chunk.next(), node_chunk.next()).await {
                Either::Left((outputs, fut)) => {
                    let outputs = outputs.unwrap_or_default();
                    let finished = match futures::poll!(fut) {
                        Poll::Ready(t) => t.expect("running is not empty"),
                        Poll::Pending => Vec::new(),
                    };
                    (outputs, finished)
                }
                Either::Right((finished, fut)) => {
                    let finished = finished.expect("running is not empty");
                    let outputs = match futures::poll!(fut) {
                        Poll::Ready(t) => t.unwrap_or_default(),
                        Poll::Pending => Vec::new(),
                    };
                    (outputs, finished)
                }
            };
        self.out_rx = output_chunk.into_inner();
        self.running = node_chunk.into_inner();
        (self, outputs, finished)
    }
}

#[derive(Debug, Default)]
pub struct FlowRunResult {
    /// Collected from flow_output nodes
    pub output: ValueSet,
    pub node_outputs: HashMap<NodeId, Vec<ValueSet>>,
    pub node_errors: HashMap<(NodeId, u32), Vec<String>>,
    /// List of nodes that didn't run (missing inputs)
    pub not_run: Vec<Uuid>,
    pub flow_errors: Vec<String>,
    pub instructions: Option<Instructions>,
}

#[derive(ThisError, Debug)]
pub enum BuildGraphError {
    #[error("2 edges connected to the same target")]
    EdgesSameTarget,
    #[error("edge's source not found: {:?}", .0)]
    EdgeSourceNotFound(NodeId),
    #[error("node not found in partial_config: {:?}", .0)]
    NodeNotFoundInPartialConfig(NodeId),
}

impl FlowGraph {
    pub async fn from_cfg(
        c: FlowConfig,
        registry: FlowRegistry,
        partial_config: Option<&PartialConfig>,
    ) -> crate::Result<Self> {
        let flow_owner = registry.flow_owner;
        let started_by = registry.started_by;
        let signer = registry.signer.clone();
        let token = registry.token.clone();
        let rhai_permit = registry.rhai_permit.clone();
        let tx_exec_config = ExecutionConfig::from_env(&c.ctx.environment)
            .inspect_err(|error| tracing::error!("error parsing ExecutionConfig: {}", error))
            .unwrap_or_default();
        tracing::debug!("execution config: {:?}", tx_exec_config);

        let ext = {
            let mut ext = Extensions::new();
            if let Some(rpc) = registry.rpc_server.clone() {
                ext.insert(rpc);
            }
            ext.insert(registry);
            ext.insert(tokio::runtime::Handle::current());
            ext
        };

        let ctx = Context::from_cfg(&c.ctx, flow_owner, started_by, signer, token, ext);

        let f = CommandFactory::new();

        let mut g = StableGraph::new();
        let mut spawned = Vec::new();

        let mut mocks = HashSet::new();
        let mut nodes = HashMap::new();
        let all_nodes = HashSet::from_iter(c.nodes.iter().map(|n| n.id));
        let only_nodes = partial_config
            .map(|c| HashSet::<NodeId>::from_iter(c.only_nodes.iter().copied()))
            .unwrap_or_else(|| all_nodes.clone());
        for id in &only_nodes {
            if !all_nodes.contains(id) {
                return Err(BuildGraphError::NodeNotFoundInPartialConfig(*id).into());
            }
        }
        let mut excluded_foreach = HashSet::new();
        for n in c.nodes {
            if n.client_node_data.r#type == CommandType::Mock {
                mocks.insert(n.id);
                continue;
            }
            if !only_nodes.contains(&n.id) {
                tracing::info!("excluding node {:?}", n.id);
                if n.command_name == crate::command::foreach::FOREACH {
                    excluded_foreach.insert(n.id);
                }
                continue;
            }
            let command = f
                .new_command(&n.command_name, &n.client_node_data, &mut spawned)
                .await?;
            let id = n.id;
            let idx = g.add_node(id);
            let node = Node {
                id,
                idx,
                form_inputs: command.read_form_data(n.form_data),
                command,
                use_previous_values: <_>::default(),
            };
            nodes.insert(id, node);
        }

        let mut edges = c.edges;
        edges.sort_by(|u, v| (&u.1, &u.0).cmp(&(&v.1, &v.0)));

        for (i, (from, to)) in edges.iter().enumerate() {
            if i > 0 && edges[i - 1].1 == *to {
                return Err(BuildGraphError::EdgesSameTarget.into());
            }

            let to_idx = match nodes.get(&to.0) {
                None => {
                    if !all_nodes.contains(&from.0) {
                        return Err(BuildGraphError::EdgeSourceNotFound(from.0).into());
                    } else if mocks.contains(&from.0) {
                        tracing::warn!("ignoring edge from mock node: {:?} -> {:?}", from, to);
                        continue;
                    } else {
                        tracing::trace!("ignoring edge from excluded node: {:?} -> {:?}", from, to);
                        continue;
                    }
                }
                Some(n) => n.idx,
            };
            let from_idx = match nodes.get(&from.0) {
                None => {
                    if !all_nodes.contains(&from.0) {
                        return Err(BuildGraphError::EdgeSourceNotFound(from.0).into());
                    } else if mocks.contains(&from.0) {
                        tracing::warn!("ignoring edge from mock node: {:?} -> {:?}", from, to);
                        continue;
                    } else {
                        nodes
                            .get_mut(&g[to_idx])
                            .unwrap()
                            .use_previous_values
                            .insert(
                                to.1.clone(),
                                UsePreviousValue {
                                    node_id: from.0,
                                    output_name: from.1.clone(),
                                    foreach: excluded_foreach.contains(&from.0),
                                },
                            );
                        continue;
                    }
                }
                Some(n) => n.idx,
            };

            g.add_edge(
                from_idx,
                to_idx,
                Edge {
                    from: from.1.clone(),
                    to: to.1.clone(),
                    values: <_>::default(),
                },
            );
        }

        Ok(Self {
            id: c.id,
            ctx,
            g,
            nodes,
            mode: c.instructions_bundling,
            output_instructions: false,
            rhai_permit,
            tx_exec_config,
            spawned,
        })
    }

    pub fn need_previous_outputs(&self) -> HashSet<NodeId> {
        self.nodes
            .values()
            .flat_map(|n| n.use_previous_values.values().map(|u| u.node_id))
            .collect()
    }

    pub fn sort_transactions(&self) -> Result<Vec<Vec<NodeId>>, anyhow::Error> {
        let nodes = petgraph::algo::toposort(&self.g, None)
            .map_err(|_| anyhow::anyhow!("graph has cycle"))?
            .into_iter()
            .map(|idx| self.g[idx])
            .collect::<Vec<_>>();

        let mut node_visited = HashSet::<NodeId>::new();
        let mut edge_visited = HashSet::<EdgeIndex>::new();
        let mut result = Vec::new();
        loop {
            let mut should_loop = false;
            let mut has_instructions = IndexMap::new();
            for n in &nodes {
                if node_visited.contains(n) {
                    continue;
                }

                let node = &self.nodes[n];

                let ready = self
                    .g
                    .edges_directed(node.idx, Direction::Incoming)
                    .all(|e| edge_visited.contains(&e.id()));
                if !ready {
                    continue;
                }
                node_visited.insert(*n);
                should_loop = true;

                let spread = match node.command.instruction_info() {
                    None => self
                        .g
                        .edges_directed(node.idx, Direction::Outgoing)
                        .map(|e| e.id())
                        .collect::<Vec<_>>(),
                    Some(info) => {
                        let before = &info.before;
                        let spread = self
                            .g
                            .edges_directed(node.idx, Direction::Outgoing)
                            .filter(|e| before.contains(&e.weight().from))
                            .map(|e| e.id())
                            .collect::<Vec<_>>();
                        has_instructions.insert(node.id, info);
                        spread
                    }
                };
                edge_visited.extend(spread);
            }

            let tx = has_instructions.keys().copied().collect::<Vec<_>>();
            if !tx.is_empty() {
                result.push(has_instructions.keys().copied().collect());

                edge_visited.extend(has_instructions.iter().flat_map(|(n, info)| {
                    let idx = self.nodes[n].idx;
                    self.g
                        .edges_directed(idx, Direction::Outgoing)
                        .filter(|e| {
                            info.signature == e.weight().from
                                || info.after.contains(&e.weight().from)
                        })
                        .map(|e| e.id())
                }));
            }

            if !should_loop {
                break;
            }
        }

        Ok(result)
    }

    fn ready(&self, idx: NodeIndex<u32>, s: &State) -> bool {
        let in_edges = || self.g.edges_directed(idx, Direction::Incoming);

        let id = self.g[idx];
        if self.nodes[&id].command.name() == crate::command::collect::COLLECT {
            let source = match in_edges().next() {
                None => return true,
                Some(e) => e.source(),
            };
            // no nested loop, so collect can only run once
            return !s.ran.contains_key(&id) && finished(self, s, source);
        }

        let filled = in_edges().all(|e| !e.weight().values.is_empty());
        if !filled {
            return false;
        }

        if s.ran.contains_key(&id) {
            // must have at least 1 input from an array
            in_edges().any(|e| {
                !matches!(
                    e.weight().values.front().unwrap().tracker,
                    TrackEdgeValue::None
                )
            })
        } else {
            true
        }
    }

    fn take_inputs(&mut self, idx: NodeIndex<u32>) -> (ValueSet, TrackEdgeValue) {
        let in_edges = self
            .g
            .edges_directed(idx, Direction::Incoming)
            .map(|e| e.id())
            .collect::<Vec<_>>();
        let mut values = ValueSet::new();
        let mut tracker = TrackEdgeValue::None;
        for edge_id in in_edges {
            let w = self.g.edge_weight_mut(edge_id).unwrap();
            let is_from_array = w
                .values
                .front()
                .expect("ready() == true")
                .tracker
                .is_array();
            let edge_value = if is_from_array {
                w.values.pop_front().unwrap()
            } else {
                w.values.front().unwrap().clone()
            };
            tracker = tracker.zip(&edge_value.tracker);
            values.insert(w.to.clone(), edge_value.value);
        }
        (values, tracker)
    }

    // For COLLECT command
    fn collect_array_input(&mut self, idx: NodeIndex<u32>) -> (ValueSet, TrackEdgeValue) {
        use crate::command::collect::ELEMENT;

        let in_edges = || self.g.edges_directed(idx, Direction::Incoming);
        let array = match in_edges().next() {
            None => Vec::new(),
            Some(e) => {
                let w = self.g.edge_weight_mut(e.id()).unwrap();
                let queue = std::mem::take(&mut w.values);
                queue.into_iter().map(|v| v.value).collect()
            }
        };

        (value::map! { ELEMENT => array }, TrackEdgeValue::None)
    }

    fn save_outputs(&mut self, o: PartialOutput, s: &mut State) {
        let info = s
            .running_info
            .get_mut(&(o.node_id, o.times))
            .expect("node must be running");

        tracing::trace!("saving {}:{}", info.id, info.command_name);

        match o.output {
            Ok((ins, mut values)) => {
                if let Some(ins_info) = &info.instruction_info {
                    if info.instruction_sent {
                        for name in &ins_info.after {
                            if !values.contains_key(name) && info.passthrough.contains_key(name) {
                                values.insert(name.clone(), info.passthrough[name].clone());
                            }
                        }
                    } else {
                        info.instruction_sent = true;
                        for name in &ins_info.before {
                            if !values.contains_key(name) && info.passthrough.contains_key(name) {
                                values.insert(name.clone(), info.passthrough[name].clone());
                            }
                        }
                    }
                } else {
                    // no instruction_info,
                    // node should only return inputs once
                    // this is the final output of the node
                    // we extend it with passthrough
                    values.extend(info.passthrough.iter().map(|(k, v)| (k.clone(), v.clone())));
                }

                let nid = info.node_idx;
                let out_edges = self
                    .g
                    .edges_directed(nid, Direction::Outgoing)
                    .map(|e| e.id())
                    .collect::<Vec<_>>();

                for eid in out_edges {
                    let w = self.g.edge_weight_mut(eid).unwrap();
                    let value = match values.get(&w.from).cloned() {
                        Some(value) => value,
                        None => continue,
                    };

                    debug_assert!(
                        w.values.is_empty()
                            || w.values.front().unwrap().tracker.is_array()
                                == info.tracker.is_array()
                    );

                    let should_loop = info.command_name == crate::command::foreach::FOREACH;

                    if should_loop && matches!(value, Value::Array(_)) {
                        let Value::Array(array) = value else {
                            unreachable!()
                        };
                        let array_iter =
                            array.into_iter().enumerate().map(|(i, value)| EdgeValue {
                                value,
                                tracker: info.tracker.nest((nid, i as u32)),
                            });

                        w.values.extend(array_iter);
                    } else {
                        w.values.push_back(EdgeValue {
                            value,
                            tracker: info.tracker.clone(),
                        });
                    }
                }

                s.event_tx
                    .unbounded_send(
                        NodeOutput {
                            time: Utc::now(),
                            node_id: info.id,
                            times: info.times,
                            output: values.clone().into(),
                        }
                        .into(),
                    )
                    .ok();
                s.result
                    .node_outputs
                    .get_mut(&info.id)
                    .expect("bug in start_node")
                    .get_mut(info.times as usize)
                    .expect("bug in start_node")
                    .extend(values);

                if ins.instructions.is_empty() {
                    o.resp.send(Ok(execute::Response { signature: None })).ok();
                } else if info.instruction_info.is_some() {
                    info.waiting = Some(Waiting {
                        instructions: ins,
                        resp: o.resp,
                    });
                } else {
                    let error = "this node should not have instructions, did you forget to define instruction_info?";
                    o.resp.send(Err(execute::Error::other(error))).ok();
                    node_error(
                        &s.event_tx,
                        &mut s.result,
                        info.id,
                        info.times,
                        error.to_owned(),
                    );
                }
            }
            Err(error) => {
                let err_str = error.to_string();
                o.resp.send(Err(error.into())).ok();
                node_error(&s.event_tx, &mut s.result, info.id, info.times, err_str);
            }
        }
    }

    fn node_finished(
        &mut self,
        join_result: Result<Finished, JoinError>,
        s: &mut State,
    ) -> Result<(), String> {
        match join_result {
            Ok(Finished {
                node,
                times,
                finished_at,
                result,
            }) => {
                let (resp, _) = oneshot::channel();
                self.save_outputs(
                    PartialOutput {
                        node_id: node.id,
                        times,
                        output: result.map(|v| (Instructions::default(), v)),
                        resp,
                    },
                    s,
                );

                let output = &s.result.node_outputs[&node.id][times as usize];
                let missing = node
                    .command
                    .outputs()
                    .iter()
                    .filter(|o| !o.optional && !output.contains_key(&o.name))
                    .map(|o| o.name.clone())
                    .collect::<Vec<String>>();
                if !missing.is_empty() {
                    node_error(
                        &s.event_tx,
                        &mut s.result,
                        node.id,
                        times,
                        format!("output not found: {:?}", missing),
                    );
                }

                tracing::trace!("node finished {}:{}", node.id, node.command.name());
                s.event_tx
                    .unbounded_send(
                        NodeFinish {
                            time: finished_at,
                            node_id: node.id,
                            times,
                        }
                        .into(),
                    )
                    .ok();
                s.running_info.remove(&(node.id, times));
                self.nodes.insert(node.id, node);
                Ok(())
            }
            Err(error) => {
                let error = if error.is_panic() {
                    format!("command panicked: {error}")
                } else {
                    "task canceled".to_owned()
                };
                s.flow_error(error.clone());
                Err(error)
            }
        }
    }

    async fn collect_and_execute_instructions(
        &mut self,
        s: &mut State,
        txs: Vec<Vec<NodeId>>,
    ) -> ControlFlow<Instructions> {
        tracing::info!("collecting instructions");
        let mut txs: Vec<IndexMap<NodeId, Option<Waiting>>> = txs
            .into_iter()
            .map(|tx| tx.into_iter().map(|id| (id, None)).collect())
            .collect();
        for info in s.running_info.values_mut() {
            let w = match info.waiting.take() {
                Some(w) => w,
                None => unreachable!("all_waiting == true"),
            };
            let tx = txs
                .iter_mut()
                .find(|tx| tx.contains_key(&info.id))
                .expect("bug in sort_transactions");
            tx[&info.id] = Some(w);
        }

        for tx in txs {
            debug_assert!(!tx.is_empty());
            let is_complete = tx.values().all(Option::is_some);
            if is_complete {
                let mut tx = tx.into_values().rev().collect::<Option<Vec<_>>>().unwrap();
                while let Some(w) = tx.pop() {
                    use std::ops::Range;
                    struct Responder {
                        sender: oneshot::Sender<Result<execute::Response, execute::Error>>,
                        range: Range<usize>,
                    }

                    let (ins, resp) = {
                        let mut ins = w.instructions;
                        if let Some(signer) = self
                            .tx_exec_config
                            .overwrite_feepayer
                            .clone()
                            .map(|x| x.to_keypair())
                        {
                            ins.set_feepayer(signer.clone_keypair());
                        }
                        let mut resp = vec![Responder {
                            sender: w.resp,
                            range: 0..ins.instructions.len(),
                        }];
                        while let Some(w) = tx.pop() {
                            let old_len = ins.instructions.len();
                            match ins.combine(w.instructions) {
                                Ok(_) => {
                                    let new_len = ins.instructions.len();
                                    resp.push(Responder {
                                        sender: w.resp,
                                        range: old_len..new_len,
                                    });
                                }
                                Err(ins) => {
                                    tx.push(Waiting {
                                        instructions: ins,
                                        resp: w.resp,
                                    });
                                    break;
                                }
                            }
                        }
                        (ins, resp)
                    };

                    // this flow is an "Interflow instructions"
                    if self.output_instructions {
                        for resp in resp {
                            resp.sender.send(Err(execute::Error::Canceled(None))).ok();
                        }
                        return ControlFlow::Break(ins);
                    }
                    let res = if s.stop.token.is_cancelled() {
                        Err(execute::Error::Canceled(s.stop.get_reason()))
                    } else if s.stop_shared.token.is_cancelled() {
                        Err(execute::Error::Canceled(s.stop_shared.get_reason()))
                    } else {
                        tracing::info!("executing instructions");
                        let config = self.tx_exec_config.clone();
                        s.stop
                            .race(
                                std::pin::pin!(s.stop_shared.race(
                                    std::pin::pin!(ins.execute(
                                        &self.ctx.solana_client,
                                        self.ctx.signer.clone(),
                                        Some(s.flow_run_id),
                                        config,
                                    )),
                                    execute::Error::Canceled,
                                )),
                                execute::Error::Canceled,
                            )
                            .await
                    };

                    let res = res.map(|s| execute::Response { signature: Some(s) });
                    let failed_instruction = res.as_ref().err().and_then(|e| match e {
                        execute::Error::Solana { error, inserted } => {
                            find_failed_instruction(error)
                                .and_then(|pos| pos.checked_sub(*inserted))
                        }
                        _ => None,
                    });
                    for resp in resp {
                        if let Some(pos) = failed_instruction {
                            if resp.range.contains(&pos) {
                                resp.sender.send(res.clone()).ok();
                            } else {
                                debug_assert!(res.is_err());
                                resp.sender.send(Err(execute::Error::TxSimFailed)).ok();
                            }
                        } else {
                            resp.sender.send(res.clone()).ok();
                        }
                    }

                    if let Err(error) = &res {
                        for w in tx.into_iter().rev() {
                            w.resp.send(Err(error.clone())).ok();
                        }
                        break;
                    }
                }
            } else {
                // Some nodes didn't output their instructions
                for v in tx.into_values().flatten() {
                    v.resp.send(Err(execute::Error::TxIncomplete)).ok();
                }
            }
        }
        ControlFlow::Continue(())
    }

    fn supply_partial_run_values(&mut self, fake_node: NodeIndex<u32>, s: &mut State) {
        let out_edges = self
            .g
            .edges_directed(fake_node, Direction::Outgoing)
            .map(|e| (e.id(), e.target()))
            .collect::<Vec<_>>();
        for (eid, target) in out_edges {
            let target_id = self.g[target];
            let w = self.g.edge_weight_mut(eid).unwrap();
            let (node_id, output_name) = w.from.split_once('/').expect("separated by /");
            let node_id: Uuid = node_id.parse().expect("UUID");
            let outputs = s
                .previous_values
                .get(&node_id)
                .expect("checked in `FlowGraph::run`");
            let mut i = 0;
            // run_fake is run before all other nodes, so this won't
            // panick
            let foreach = self.nodes[&target_id].use_previous_values[&w.to].foreach;
            let use_element = outputs.len() > 1;
            for map in outputs {
                let value = match map {
                    Value::Map(map) => match map.get(output_name) {
                        Some(value) => value.clone(),
                        None => {
                            tracing::warn!("value not found for port {:?}", output_name);
                            continue;
                        }
                    },
                    _ => {
                        tracing::warn!("expecting map");
                        continue;
                    }
                };
                if foreach {
                    if let Value::Array(array) = value {
                        for value in array {
                            w.values.push_back(EdgeValue {
                                value,
                                tracker: TrackEdgeValue::Element(fake_node, i as u32),
                            });
                            i += 1;
                        }
                    } else {
                        tracing::error!("expecting array: {:?}", w.from);
                    }
                } else {
                    let tracker = if use_element {
                        let t = TrackEdgeValue::Element(fake_node, i as u32);
                        i += 1;
                        t
                    } else {
                        TrackEdgeValue::None
                    };
                    w.values.push_back(EdgeValue { value, tracker });
                }
            }
            tracing::trace!("{} values for fake edge {}:{}", i, node_id, output_name);
        }
    }

    pub async fn run(
        &mut self,
        event_tx: EventSender,
        flow_run_id: FlowRunId,
        flow_inputs: value::Map,
        stop: StopSignal,
        stop_shared: StopSignal,
        previous_values: HashMap<NodeId, Vec<Value>>,
    ) -> FlowRunResult {
        event_tx
            .unbounded_send(FlowStart { time: Utc::now() }.into())
            .ok();

        let (out_tx, out_rx) = mpsc::unbounded::<PartialOutput>();
        let mut s = State {
            flow_run_id,
            previous_values,
            flow_inputs,
            event_tx,
            ran: HashMap::new(),
            running: FuturesUnordered::new(),
            running_info: HashMap::new(),
            result: FlowRunResult::default(),
            stop,
            stop_shared,
            out_tx,
            out_rx,
        };

        let txs = self.sort_transactions();

        match &txs {
            Ok(txs) => {
                let txs = txs
                    .iter()
                    .map(|tx| {
                        tx.iter()
                            .map(|id| {
                                let name =
                                    self.nodes.get(id).expect("node not found").command.name();
                                format!("{}:{}", id, name)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                tracing::trace!("transactions: {:?}", txs);
            }
            Err(error) => {
                s.flow_error(format!("sort_transactions failed: {}", error));
                s.stop.token.cancel();
            }
        }

        let fake_node = self.g.add_node(Uuid::nil());
        for n in self.nodes.values() {
            for (
                input_name,
                UsePreviousValue {
                    node_id,
                    output_name,
                    ..
                },
            ) in &n.use_previous_values
            {
                if s.previous_values.contains_key(node_id) {
                    self.g.add_edge(
                        fake_node,
                        n.idx,
                        Edge {
                            from: format!("{}/{}", node_id, output_name),
                            to: input_name.clone(),
                            values: <_>::default(),
                        },
                    );
                } else {
                    // TODO: more diagnostic info
                    s.flow_error(format!("no value for port {:?}", input_name));
                    s.stop.token.cancel();
                }
            }
        }
        self.supply_partial_run_values(fake_node, &mut s);

        // TODO: is this the best way to do this
        match Arc::get_mut(&mut self.ctx.extensions) {
            Some(ext) => {
                ext.insert(s.event_tx.clone());
                ext.insert(s.stop.token.clone());
            }
            None => {
                tracing::error!("could not insert to extensions, this is a bug");
            }
        }

        'LOOP: loop {
            tracing::trace!("new round");
            if s.stop.token.is_cancelled() {
                break;
            }
            let nodes = self.g.node_indices().collect::<Vec<_>>();
            let mut started_new_nodes = false;
            for idx in nodes {
                let id = self.g.node_weight(idx).unwrap();

                if !self.nodes.contains_key(id) {
                    continue;
                }

                if self.ready(idx, &s) {
                    self.start_node(*id, &mut s);
                    started_new_nodes = true;
                }
            }

            if s.running.is_empty() {
                break 'LOOP;
            }

            let all_waiting = s.running_info.values().all(|i| i.waiting.is_some());
            if !started_new_nodes && all_waiting {
                let txs = match &txs {
                    Ok(txs) => txs.clone(),
                    Err(error) => {
                        s.flow_error(error.to_string());
                        s.stop.token.cancel();
                        continue;
                    }
                };

                if let ControlFlow::Break(ins) =
                    self.collect_and_execute_instructions(&mut s, txs).await
                {
                    s.result.instructions = Some(ins);
                    s.stop.token.cancel();
                    continue;
                }
            }

            let (outputs, finished);
            (s, outputs, finished) = s.wait().await;

            for output in outputs {
                self.save_outputs(output, &mut s);
            }
            for join_result in finished {
                if let Err(error) = self.node_finished(join_result, &mut s) {
                    tracing::trace!("{}, stopping flow", error);
                    break 'LOOP;
                }
            }
        }

        for id in self.nodes.keys() {
            if !s.ran.contains_key(id) {
                s.result.not_run.push(*id);
            }
        }

        self.collect_flow_output(&mut s);

        let failed = s
            .result
            .node_errors
            .iter()
            .filter(|(_, e)| !e.is_empty())
            .count();
        if failed > 0 {
            s.flow_error(format!("{} nodes failed", failed));
        }

        s.event_tx
            .unbounded_send(
                FlowFinish {
                    time: Utc::now(),
                    output: s.result.output.clone().into(),
                    not_run: s.result.not_run.clone(),
                }
                .into(),
            )
            .ok();

        self.g.remove_node(fake_node);

        s.result
    }

    fn collect_flow_output(&self, s: &mut State) {
        for (id, n) in self
            .nodes
            .iter()
            .filter(|(_, n)| n.command.name() == FLOW_OUTPUT)
        {
            let name = &n.command.outputs()[0].name;
            if let Some(values) = s.result.node_outputs.get(id) {
                let mut values = values
                    .iter()
                    .filter_map(|o| o.get(name))
                    .cloned()
                    .collect::<Vec<Value>>();
                let value = match values.len() {
                    0 => continue,
                    1 => values.pop().unwrap(),
                    _ => Value::Array(values),
                };
                s.result.output.insert(name.clone(), value);
            }
        }
    }

    fn start_node(&mut self, id: NodeId, s: &mut State) {
        let node = self.nodes.remove(&id).unwrap();
        let idx = node.idx;
        let times = *s.ran.entry(node.id).and_modify(|t| *t += 1).or_insert(0);

        let (mut inputs, tracker) = match node.command.name().as_str() {
            crate::command::flow_input::FLOW_INPUT => (s.flow_inputs.clone(), TrackEdgeValue::None),
            crate::command::collect::COLLECT => self.collect_array_input(idx),
            _ => self.take_inputs(idx),
        };

        for (name, value) in &node.form_inputs {
            // use form values if they are not supplied by edges
            inputs.entry(name.clone()).or_insert_with(|| value.clone());
        }

        s.running_info.insert(
            (node.id, times),
            RunningNodeInfo {
                id: node.id,
                times,
                node_idx: node.idx,
                command_name: node.command.name(),
                tracker,
                instruction_info: node.command.instruction_info(),
                passthrough: node.command.passthrough_outputs(&inputs),
                waiting: None,
                instruction_sent: false,
            },
        );
        let outputs = s.result.node_outputs.entry(node.id).or_default();
        debug_assert_eq!(outputs.len(), times as usize);
        outputs.push(<_>::default());
        let rhai_permit = self.rhai_permit.clone();
        let is_rhai_script = rhai_script::is_rhai_script(&node.command.name());
        let span =
            tracing::error_span!(NODE_SPAN_NAME, node_id = node.id.to_string(), times = times);
        let task = run_command(
            node,
            s.flow_run_id,
            times,
            inputs,
            self.ctx.clone(),
            s.event_tx.clone(),
            s.stop.clone(),
            s.stop_shared.clone(),
            s.out_tx.clone(),
            self.mode.clone(),
            self.tx_exec_config.clone(),
        );
        let handler = tokio::spawn(
            async move {
                if is_rhai_script {
                    let p = rhai_permit.acquire().await;
                    let result = task.await;
                    std::mem::drop(p);
                    result
                } else {
                    task.await
                }
            }
            .instrument(span),
        );

        s.running.push(handler);
    }
}

struct ExecuteWithBundling {
    node_id: NodeId,
    times: u32,
    tx: mpsc::UnboundedSender<PartialOutput>,
}

impl tower::Service<execute::Request> for ExecuteWithBundling {
    type Response = execute::Response;
    type Error = execute::Error;
    type Future = BoxFuture<'static, Result<execute::Response, execute::Error>>;
    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: execute::Request) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        self.tx
            .unbounded_send(PartialOutput {
                node_id: self.node_id,
                times: self.times,
                output: Ok((req.instructions, req.output)),
                resp: tx,
            })
            .ok();
        rx.map(|r| r?).boxed()
    }
}

struct ExecuteNoBundling {
    node_id: NodeId,
    times: u32,
    tx: mpsc::UnboundedSender<PartialOutput>,
    stop_shared: StopSignal,
    simple_svc: execute::Svc,
    overwrite_feepayer: Option<Keypair>,
}

impl tower::Service<execute::Request> for ExecuteNoBundling {
    type Response = execute::Response;
    type Error = execute::Error;
    type Future = BoxFuture<'static, Result<execute::Response, execute::Error>>;
    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, mut req: execute::Request) -> Self::Future {
        use tower::ServiceExt;
        if req.instructions.instructions.is_empty() {
            // empty instructions wont be bundled,
            // so just process like normal
            let (tx, rx) = oneshot::channel();
            self.tx
                .unbounded_send(PartialOutput {
                    node_id: self.node_id,
                    times: self.times,
                    output: Ok((req.instructions, req.output)),
                    resp: tx,
                })
                .ok();
            rx.map(|r| r?).boxed()
        } else {
            if self.stop_shared.token.is_cancelled() {
                let reason = self.stop_shared.get_reason();
                return Box::pin(async move { Err(execute::Error::Canceled(reason)) });
            }
            // execute before sending the partial output
            let mut svc = self.simple_svc.clone();
            let tx = self.tx.clone();
            let node_id = self.node_id;
            let times = self.times;
            let output = req.output.clone();
            let overwrite_feepayer = self.overwrite_feepayer.as_ref().map(|k| k.clone_keypair());
            let task = async move {
                if let Some(signer) = overwrite_feepayer {
                    req.instructions.set_feepayer(signer);
                }
                let res = svc.ready().await?.call(req).await;
                let output = match &res {
                    Ok(_) => Ok((Instructions::default(), output)),
                    Err(error) => Err(error.clone().into()),
                };
                // send output with empty instructions after we've executed them
                // only send on Ok, because the node should send Err by itself
                if output.is_ok() {
                    let (resp, rx) = oneshot::channel();
                    tx.unbounded_send(PartialOutput {
                        node_id,
                        times,
                        output,
                        resp,
                    })
                    .ok();
                    rx.await.ok();
                }
                res
            }
            .boxed();
            let stop = self.stop_shared.clone();
            Box::pin(async move { stop.race(task, execute::Error::Canceled).await })
        }
    }
}

struct Finished {
    node: Node,
    times: u32,
    finished_at: DateTime<Utc>,
    result: Result<value::Map, CommandError>,
}

#[allow(clippy::too_many_arguments)]
async fn run_command(
    node: Node,
    flow_run_id: FlowRunId,
    times: u32,
    inputs: value::Map,
    mut ctx: Context,
    event_tx: EventSender,
    stop: StopSignal,
    stop_shared: StopSignal,
    tx: mpsc::UnboundedSender<PartialOutput>,
    mode: client::BundlingMode,
    tx_exec_config: ExecutionConfig,
) -> Finished {
    let svc = match mode {
        client::BundlingMode::Off => TowerClient::from_service(
            ExecuteNoBundling {
                node_id: node.id,
                times,
                tx: tx.clone(),
                simple_svc: execute::simple(&ctx, 32, Some(flow_run_id), tx_exec_config.clone()),
                stop_shared,
                overwrite_feepayer: tx_exec_config
                    .overwrite_feepayer
                    .clone()
                    .map(|x| x.to_keypair()),
            },
            execute::Error::worker,
            32,
        ),
        client::BundlingMode::Automatic => TowerClient::from_service(
            ExecuteWithBundling {
                node_id: node.id,
                times,
                tx: tx.clone(),
            },
            execute::Error::worker,
            32,
        ),
    };
    ctx.command = Some(CommandContext {
        svc,
        flow_run_id,
        node_id: node.id,
        times,
    });
    if !node.command.permissions().user_tokens {
        ctx.get_jwt = get_jwt::not_allowed();
    }

    event_tx
        .unbounded_send(
            NodeStart {
                time: Utc::now(),
                node_id: node.id,
                times,
                input: inputs.clone().into(),
            }
            .into(),
        )
        .ok();

    tracing::trace!("starting node {}:{}", node.id, node.command.name());
    let result = stop
        .race(node.command.run(ctx, inputs), |reason| {
            crate::Error::Canceled(reason).into()
        })
        .await;

    Finished {
        node,
        times,
        result,
        finished_at: Utc::now(),
    }
}

fn finished(f: &FlowGraph, s: &State, nid: NodeIndex<u32>) -> bool {
    let mut visited = HashSet::new();

    finished_recursive(f, s, nid, &mut visited)
}

fn finished_recursive(
    f: &FlowGraph,
    s: &State,
    nid: NodeIndex<u32>,
    visited: &mut HashSet<NodeIndex<u32>>,
) -> bool {
    if !visited.insert(nid) {
        return false;
    }

    if !f.nodes.contains_key(&f.g[nid]) {
        // running
        return false;
    }

    let ran = s.ran.contains_key(&f.g[nid]);

    let in_edges = || f.g.edges_directed(nid, Direction::Incoming);

    if in_edges().count() == 0 {
        return ran;
    }

    let mut has_array_input = false;
    let mut filled = true;
    let mut all_sources_not_finished = true;

    for e in in_edges() {
        if let Some(value) = e.weight().values.front() {
            let is_from_array = value.tracker.is_array();
            has_array_input |= is_from_array;
        } else {
            let source_finished = finished_recursive(f, s, e.source(), visited);
            all_sources_not_finished &= !source_finished;
            filled = false;
        }
    }

    if filled {
        if has_array_input {
            false
        } else {
            ran
        }
    } else {
        !all_sources_not_finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow_run_events::event_channel;
    use anyhow::anyhow;
    use flow_lib::config::client::ClientConfig;

    use cmds_solana as _;
    use cmds_std as _;

    #[derive(serde::Deserialize)]
    struct TestFile {
        flow: ClientConfig,
    }

    #[tokio::test]
    async fn test_stop() {
        let task = async {
            tokio::time::sleep(Duration::from_secs(4)).await;
            Ok::<_, anyhow::Error>(())
        };
        let first = StopSignal::new();
        let second = StopSignal::new();

        tokio::spawn({
            let second = second.clone();
            async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                second.stop(0, None);
            }
        });

        let error = first
            .race(
                std::pin::pin!(second.race(std::pin::pin!(task), |_| anyhow!("second"))),
                |_| anyhow!("first"),
            )
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(error, "second");
    }

    #[tokio::test]
    async fn test_foreach_nested() {
        let json = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_files/2_foreach.json"
        ));
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;
        assert_eq!(
            res.output["output"],
            Value::Array(
                [
                    Value::String("0,0".to_owned()),
                    Value::String("0,1".to_owned()),
                    Value::String("1,0".to_owned()),
                    Value::String("1,1".to_owned()),
                ]
                .to_vec()
            )
        );
    }

    #[tokio::test]
    async fn test_uneven_loop() {
        let json = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_files/uneven_loop.json"
        ));
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;

        assert_eq!(
            res.output["1"],
            Value::Array([Value::U64(1), Value::U64(2), Value::U64(3),].to_vec())
        );
        assert_eq!(
            res.output["2"],
            Value::Array(
                [
                    Value::String("0,0".to_owned()),
                    Value::String("0,1".to_owned()),
                    Value::String("1,0".to_owned()),
                ]
                .to_vec()
            )
        );
    }

    /*
     * // TODO: a node in this flow changed
    #[tokio::test]
    async fn test_collect_instructions() {
        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/test_files/nft.json"));
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();

        let mut txs = flow.sort_transactions().unwrap();
        assert_eq!(txs.len(), 1);
        let tx = txs.pop().unwrap();
        let mut names = Vec::new();
        for id in tx {
            let name = flow.nodes[&id].command.name();
            names.push(name);
        }
        let expected = [
            "create_mint_account",
            "associated_token_account",
            "mint_token",
            "create_metadata_account",
            "create_master_edition",
        ];
        assert_eq!(names, expected);
    }
    */
}
