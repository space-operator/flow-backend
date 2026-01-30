use crate::{command::flow_output::FLOW_OUTPUT, flow_registry::FlowRegistry};
use base64::prelude::*;
use chrono::{DateTime, Utc};
use command_rpc::flow_side::command_factory::CommandFactoryWithRemotes;
use flow_lib::{
    CommandType, FlowConfig, FlowId, FlowRunId, Name, NodeId, ValueSet,
    command::{
        CommandError, CommandFactory, CommandTrait, InstructionInfo, input_is_required,
        keypair_outputs, output_is_optional, passthrough_outputs,
    },
    config::client::{self, PartialConfig},
    context::{
        CommandContext, CommandContextData, FlowContextData, FlowServices, FlowSetContextData,
        FlowSetServices, execute, get_jwt,
    },
    flow_run_events::{
        EventSender, FlowError, FlowFinish, FlowStart, NODE_SPAN_NAME, NodeError, NodeFinish,
        NodeLogSender, NodeOutput, NodeStart,
    },
    solana::{ExecutionConfig, Instructions, Pubkey, Wallet},
    utils::{Extensions, TowerClient, tower_client::CommonErrorExt},
};
use flow_lib_solana::{InstructionsExt, find_failed_instruction, simple_execute_svc};
use futures::{
    FutureExt, StreamExt,
    channel::{mpsc, oneshot},
    future::{BoxFuture, Either},
    stream::FuturesUnordered,
};
use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;
use petgraph::{
    Directed, Direction,
    csr::DefaultIx,
    graph::EdgeIndex,
    stable_graph::{Edges, NodeIndex, StableGraph},
    visit::{Bfs, EdgeRef, GraphRef, VisitMap, Visitable},
};
use solana_system_interface::instruction::transfer_many;
use std::{
    collections::{BTreeSet, VecDeque},
    ops::ControlFlow,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering},
    },
    task::Poll,
    time::Duration,
};
use thiserror::Error as ThisError;
use tokio::{
    sync::Semaphore,
    task::{JoinError, JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tower::{Service, ServiceExt, service_fn};
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
    pub ctx_data: FlowContextData,
    pub ctx_svcs: FlowServices,
    pub get_jwt: get_jwt::Svc,
    pub g: StableGraph<NodeId, Edge>,
    pub nodes: HashMap<NodeId, Node>,
    pub mode: client::BundlingMode,
    pub output_instructions: bool,
    pub action_identity: Option<Pubkey>,
    pub rhai_permit: Arc<Semaphore>,
    pub tx_exec_config: ExecutionConfig,
    pub parent_flow_execute: Option<execute::Svc>,
    pub fees: Vec<(Pubkey, u64)>,
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
    pub output: Result<(Option<Instructions>, ValueSet), CommandError>,
    pub resp: oneshot::Sender<Result<execute::Response, execute::Error>>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: Name,
    pub to: Name,
    pub is_required_input: bool,
    pub is_optional_output: bool,
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
    value: Option<Value>,
    tracker: TrackEdgeValue,
}

#[derive(Debug)]
struct State {
    early_return: bool,
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
    keypair_outputs: Vec<String>,
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
    #[error("node {:?}:{} has no input {:?}", .0.0, .1, .0.1)]
    NoInput((NodeId, String), String),
    #[error("node {:?}:{} has no output {:?}", .0.0, .1, .0.1)]
    NoOutput((NodeId, String), String),
}

fn remove_wallet_token(v: &mut value::Map, keypair_outputs: &[String]) {
    for o in keypair_outputs {
        if let Some(v) = v.get_mut(o)
            && let Value::Map(v) = v
        {
            v.swap_remove("token");
        }
    }
}

impl FlowGraph {
    pub async fn from_cfg(
        c: FlowConfig,
        registry: FlowRegistry,
        partial_config: Option<&PartialConfig>,
    ) -> crate::Result<Self> {
        let rhai_permit = registry.rhai_permit.clone();
        let tx_exec_config = ExecutionConfig::from_env(&c.ctx.environment)
            .inspect_err(|error| tracing::error!("error parsing ExecutionConfig: {}", error))
            .unwrap_or_default();
        let parent_flow_execute = registry.parent_flow_execute.clone();
        tracing::debug!("execution config: {:?}", tx_exec_config);
        let get_jwt = registry.backend.token.clone();

        let ctx_data = FlowContextData {
            environment: c.ctx.environment,
            flow_run_id: FlowRunId::nil(),
            inputs: ValueSet::default(),
            set: FlowSetContextData {
                endpoints: c.ctx.endpoints,
                flow_owner: registry.flow_owner,
                started_by: registry.started_by,
                http: c.ctx.http_client,
                solana: c.ctx.solana_client,
            },
        };
        let ctx_svcs = FlowServices {
            signer: registry.backend.signer.clone(),
            set: FlowSetServices {
                api_input: registry.backend.api_input.clone(),
                extensions: Arc::new({
                    let mut ext = Extensions::new();
                    if let Some(rpc) = registry.rpc_server.clone() {
                        ext.insert(rpc);
                    }
                    ext.insert(registry.make_run_rhai_svc());
                    ext.insert(registry.make_start_flow_svc());
                    ext.insert(tokio::runtime::Handle::current());
                    ext.insert(crate::command::wallet::WalletPermit::new());
                    ext
                }),
                http: registry.http.clone(),
                solana_client: Arc::new(
                    ctx_data
                        .set
                        .solana
                        .build_client(Some(registry.http.clone())),
                ),
                helius: registry.backend.helius.clone(),
            },
        };

        let f = CommandFactoryWithRemotes {
            factory: CommandFactory::collect(),
            remotes: registry.remotes,
        };

        let mut g = StableGraph::new();

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
        let mut join_set = JoinSet::new();
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

            let mut f = f.clone();
            let task = async move {
                let command = f
                    .init(&n.client_node_data)
                    .await
                    .map_err(crate::Error::CreateCmd)?
                    .ok_or_else(|| {
                        crate::Error::CreateCmd(CommandError::msg(format!(
                            "not found: {}",
                            n.client_node_data.node_id
                        )))
                    })?;
                Ok::<_, crate::Error>((n, command))
            };
            join_set.spawn_local(task);
        }

        let results = join_set.join_all().await;
        for result in results {
            let (n, command) = result?;
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

            let (to_idx, required) = match nodes.get(&to.0) {
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
                Some(n) => {
                    let required = input_is_required(&*n.command, &to.1)
                        .ok_or_else(|| BuildGraphError::NoInput(to.clone(), n.command.name()))?;
                    (n.idx, required)
                }
            };
            let (from_idx, optional) = match nodes.get(&from.0) {
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
                Some(n) => {
                    let optional = output_is_optional(&*n.command, &from.1)
                        .ok_or_else(|| BuildGraphError::NoOutput(from.clone(), n.command.name()))?;
                    (n.idx, optional)
                }
            };

            g.add_edge(
                from_idx,
                to_idx,
                Edge {
                    from: from.1.clone(),
                    to: to.1.clone(),
                    values: <_>::default(),
                    is_required_input: required,
                    is_optional_output: optional,
                },
            );
        }

        Ok(Self {
            id: c.id,
            ctx_data,
            ctx_svcs,
            get_jwt,
            g,
            nodes,
            mode: c.instructions_bundling,
            output_instructions: false,
            action_identity: None,
            fees: Vec::new(),
            rhai_permit,
            tx_exec_config,
            parent_flow_execute,
        })
    }

    pub fn get_interflow_instruction_info(&self) -> crate::Result<InstructionInfo> {
        let mut txs = self.sort_transactions()?;
        if txs.len() != 1 {
            return Err(crate::Error::NeedOneTx);
        }
        let tx = txs.pop().unwrap();

        // find siganture output
        let mut signature = tx
            .iter()
            .filter_map(|node_id| {
                let idx = self.nodes[node_id].idx;
                let mut flow_output = self
                    .out_edges(idx)
                    .map(|e| self.g[e.target()])
                    .filter_map(|node_id| {
                        let cmd = &self.nodes[&node_id].command;
                        if cmd.name() == FLOW_OUTPUT {
                            cmd.outputs().pop().map(|o| o.name)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                if flow_output.len() != 1 {
                    return None;
                }
                Some(flow_output.pop().expect("flow_output.len() != 1"))
            })
            .collect::<Vec<_>>();
        if signature.len() != 1 {
            return Err(crate::Error::NeedOneSignatureOutput);
        }
        let signature = signature.pop().unwrap();

        // find after outputs
        let g = self.g.filter_map(
            |_, n| Some(n),
            |eid, e| {
                let (source, _) = self.g.edge_endpoints(eid)?;
                let source_id = self.g[source];
                if tx.contains(&source_id) {
                    let info = self.nodes[&source_id]
                        .command
                        .instruction_info()
                        .expect("node is in tx");
                    if !(info.signature == e.from || info.after.contains(&e.from)) {
                        return None;
                    }
                }
                Some(Edge {
                    from: e.from.clone(),
                    to: e.to.clone(),
                    is_required_input: e.is_required_input,
                    is_optional_output: e.is_optional_output,
                    values: <_>::default(),
                })
            },
        );
        let mut bfs = new_bfs(&g, tx.iter().map(|id| self.nodes[id].idx));
        let mut after = Vec::new();
        while let Some(nid) = bfs.next(&g) {
            let node = &self.nodes[g[nid]];
            if node.command.name() == FLOW_OUTPUT {
                let label = match node.command.outputs().pop() {
                    Some(label) => label.name,
                    None => continue,
                };
                if label != signature {
                    after.push(label);
                }
            }
        }

        let before = self
            .nodes
            .values()
            .filter_map(|n| {
                (n.command.name() == FLOW_OUTPUT)
                    .then(|| n.command.outputs().pop().map(|o| o.name))
                    .flatten()
            })
            .filter(|name| name != &signature && !after.contains(name))
            .collect();

        Ok(InstructionInfo {
            signature,
            before,
            after,
        })
    }

    pub fn need_previous_outputs(&self) -> HashSet<NodeId> {
        self.nodes
            .values()
            .flat_map(|n| n.use_previous_values.values().map(|u| u.node_id))
            .collect()
    }

    pub fn sort_transactions(&self) -> crate::Result<Vec<Vec<NodeId>>> {
        let nodes = petgraph::algo::toposort(&self.g, None)
            .map_err(|_| crate::Error::Cycle)?
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

    fn in_edges(&self, idx: NodeIndex<u32>) -> Edges<'_, Edge, Directed, DefaultIx> {
        self.g.edges_directed(idx, Direction::Incoming)
    }

    fn out_edges(&self, idx: NodeIndex<u32>) -> Edges<'_, Edge, Directed, DefaultIx> {
        self.g.edges_directed(idx, Direction::Outgoing)
    }

    fn submit_is_false(&self, idx: NodeIndex<u32>) -> bool {
        self.in_edges(idx)
            .find_map(|e| (e.weight().to == "submit").then(|| e.weight().values.front()))
            .flatten()
            .map(|v| v.value == Some(Value::Bool(false)))
            .unwrap_or(false)
    }

    fn ready(&self, idx: NodeIndex<u32>, s: &State) -> bool {
        let id = self.g[idx];
        let cmd = &self.nodes[&id].command;
        if cmd.instruction_info().is_some() {
            // Don't start node if it has a `false` submit input
            if self.submit_is_false(idx) {
                return false;
            }
        }
        if cmd.name() == crate::command::collect::COLLECT {
            let source = match self.in_edges(idx).next() {
                None => return true,
                Some(e) => e.source(),
            };
            // no nested loop, so collect can only run once
            return !s.ran.contains_key(&id) && finished(self, s, source);
        }

        let filled = self.in_edges(idx).all(|e| {
            let w = e.weight();
            w.values
                .front()
                .map(|EdgeValue { value, .. }| value.is_some() || !w.is_required_input)
                .unwrap_or_else(|| !w.is_required_input && finished(self, s, e.source()))
        });
        if !filled {
            return false;
        }

        if s.ran.contains_key(&id) {
            // must have at least 1 input from an array
            self.in_edges(idx).any(|e| {
                e.weight()
                    .values
                    .front()
                    .map(|v| v.tracker.is_array())
                    .unwrap_or(false)
            })
        } else {
            true
        }
    }

    fn take_inputs(&mut self, idx: NodeIndex<u32>) -> (ValueSet, TrackEdgeValue) {
        let in_edges = self.in_edges(idx).map(|e| e.id()).collect::<Vec<_>>();
        let mut values = ValueSet::new();
        let mut tracker = TrackEdgeValue::None;
        for edge_id in in_edges {
            let w = self.g.edge_weight_mut(edge_id).unwrap();
            if let Some(EdgeValue {
                value: Some(value),
                tracker: edge_tracker,
            }) = w.values.front()
            {
                let is_from_array = edge_tracker.is_array();
                tracker = tracker.zip(edge_tracker);
                values.insert(w.to.clone(), value.clone());
                if is_from_array {
                    w.values.pop_front();
                }
            }
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
                queue.into_iter().filter_map(|v| v.value).collect()
            }
        };

        (value::map! { ELEMENT => array }, TrackEdgeValue::None)
    }

    fn save_missing_optional_outputs(&mut self, node_id: NodeId, times: u32, s: &mut State) {
        let info = s
            .running_info
            .get(&(node_id, times))
            .expect("node must be running");

        tracing::trace!("saving {}:{}", info.id, info.command_name);

        let output = &s.result.node_outputs[&node_id][times as usize];
        let edges = self
            .out_edges(info.node_idx)
            .filter_map(|e| {
                let w = e.weight();
                (w.is_optional_output && !output.contains_key(&w.from)).then_some(e.id())
            })
            .collect::<Vec<_>>();
        for eid in edges {
            let w = self.g.edge_weight_mut(eid).unwrap();
            w.values.push_back(EdgeValue {
                value: None,
                tracker: info.tracker.clone(),
            });
        }
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
                                value: Some(value),
                                tracker: info.tracker.nest((nid, i as u32)),
                            });

                        w.values.extend(array_iter);
                    } else {
                        w.values.push_back(EdgeValue {
                            value: Some(value),
                            tracker: info.tracker.clone(),
                        });
                    }
                }

                remove_wallet_token(&mut values, &info.keypair_outputs);

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

                if ins.is_none() {
                    o.resp.send(Ok(execute::Response { signature: None })).ok();
                } else if info.instruction_info.is_some() {
                    info.waiting = Some(Waiting {
                        instructions: ins.expect("ins.is_none() == false"),
                        resp: o.resp,
                    });
                } else {
                    let error = "this node should not have instructions, did you forget to define instruction_info?";
                    o.resp.send(Err(execute::Error::msg(error))).ok();
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
                if let Some(execute_error) = error.downcast_ref::<execute::Error>()
                    && matches!(execute_error, execute::Error::Collected)
                {
                    return;
                }
                let err_str = error.to_string();
                o.resp.send(Err(execute::Error::from_anyhow(error))).ok();
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
                        output: result.map(|v| (None, v)),
                        resp,
                    },
                    s,
                );

                self.save_missing_optional_outputs(node.id, times, s);

                let output = &s.result.node_outputs[&node.id][times as usize];
                let missing = node
                    .command
                    .outputs()
                    .into_iter()
                    .filter_map(|o| {
                        (!o.optional && !output.contains_key(&o.name)).then_some(o.name)
                    })
                    .collect::<Vec<String>>();
                if !missing.is_empty() && !s.early_return {
                    node_error(
                        &s.event_tx,
                        &mut s.result,
                        node.id,
                        times,
                        format!("output not found: {missing:?}"),
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
    ) -> ControlFlow<Result<Instructions, String>> {
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
            let tx = txs.iter_mut().find(|tx| tx.contains_key(&info.id));
            let tx = match tx {
                Some(tx) => tx,
                None => {
                    return ControlFlow::Break(Err(format!(
                        "could not find transaction position of {}:{}",
                        info.id, info.command_name
                    )));
                }
            };
            tx[&info.id] = Some(w);
        }

        for tx in txs {
            debug_assert!(!tx.is_empty());
            let is_complete = tx.iter().all(|(node_id, wait)| {
                match self.nodes.get(node_id) {
                    Some(node) => {
                        // node is not running
                        self.submit_is_false(node.idx)
                    }
                    None => {
                        // none mean it is running
                        wait.is_some()
                    }
                }
            });
            if is_complete {
                let mut tx = tx.into_values().rev().flatten().collect::<Vec<_>>();
                while let Some(w) = tx.pop() {
                    use std::ops::Range;
                    struct Responder {
                        sender: oneshot::Sender<Result<execute::Response, execute::Error>>,
                        range: Range<usize>,
                    }

                    let (mut ins, resp) = {
                        let mut ins = w.instructions;
                        if let Some(signer) = self
                            .tx_exec_config
                            .overwrite_feepayer
                            .clone()
                            .map(|x| x.to_keypair())
                        {
                            ins.set_feepayer(signer);
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
                    if !self.fees.is_empty() {
                        ins.combine(Instructions {
                            lookup_tables: None,
                            fee_payer: ins.fee_payer,
                            signers: vec![],
                            instructions: transfer_many(&ins.fee_payer, &self.fees),
                        })
                        .expect("same fee payer");
                    }

                    // this flow is an "Interflow instructions"
                    if self.output_instructions {
                        for resp in resp {
                            resp.sender.send(Err(execute::Error::Collected)).ok();
                        }
                        s.early_return = true;
                        return ControlFlow::Break(Ok(ins));
                    }
                    let res = if s.stop.token.is_cancelled() {
                        Err(execute::Error::Canceled(s.stop.get_reason()))
                    } else if s.stop_shared.token.is_cancelled() {
                        Err(execute::Error::Canceled(s.stop_shared.get_reason()))
                    } else if ins.instructions.is_empty() {
                        Ok(execute::Response { signature: None })
                    } else if let Some(exec) = &self.parent_flow_execute {
                        self.collect_flow_output(s).await;
                        match exec.clone().ready().await {
                            Ok(exec) => {
                                s.stop
                                    .race(
                                        std::pin::pin!(s.stop_shared.race(
                                            std::pin::pin!(exec.call(execute::Request {
                                                instructions: ins,
                                                output: s.result.output.clone(),
                                            })),
                                            execute::Error::Canceled,
                                        )),
                                        execute::Error::Canceled,
                                    )
                                    .await
                            }
                            Err(error) => Err(error),
                        }
                    } else {
                        tracing::info!("executing instructions");
                        let config = self.tx_exec_config.clone();
                        let network = self.ctx_data.set.solana.cluster;
                        s.stop
                            .race(
                                std::pin::pin!(s.stop_shared.race(
                                    std::pin::pin!(ins.execute(
                                        &self.ctx_svcs.set.solana_client,
                                        self.ctx_svcs.set.helius.as_deref(),
                                        network,
                                        self.ctx_svcs.signer.clone(),
                                        Some(s.flow_run_id),
                                        config,
                                    )),
                                    execute::Error::Canceled,
                                )),
                                execute::Error::Canceled,
                            )
                            .await
                            .map(|signature| execute::Response {
                                signature: Some(signature),
                            })
                    };

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
            .out_edges(fake_node)
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
                                value: Some(value),
                                tracker: TrackEdgeValue::Element(fake_node, i as u32),
                            });
                            i += 1;
                        }
                    } else {
                        tracing::error!("expecting array: {:?}", w.from);
                    }
                } else {
                    let tracker = if use_element {
                        TrackEdgeValue::Element(fake_node, i as u32)
                    } else {
                        TrackEdgeValue::None
                    };
                    w.values.push_back(EdgeValue {
                        value: Some(value),
                        tracker,
                    });
                    i += 1;
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
        self.ctx_data.inputs = flow_inputs.clone();
        self.ctx_data.flow_run_id = flow_run_id;

        event_tx
            .unbounded_send(FlowStart { time: Utc::now() }.into())
            .ok();

        let (out_tx, out_rx) = mpsc::unbounded::<PartialOutput>();
        let mut s = State {
            early_return: false,
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
                                format!("{id}:{name}")
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                tracing::trace!("transactions: {:?}", txs);
            }
            Err(error) => {
                s.flow_error(format!("sort_transactions failed: {error}"));
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
                    let is_required_input =
                        input_is_required(&*n.command, input_name).unwrap_or(true);
                    self.g.add_edge(
                        fake_node,
                        n.idx,
                        Edge {
                            from: format!("{node_id}/{output_name}"),
                            to: input_name.clone(),
                            values: <_>::default(),
                            is_required_input,
                            is_optional_output: !is_required_input,
                        },
                    );
                } else {
                    // TODO: more diagnostic info
                    s.flow_error(format!("no value for port {input_name:?}"));
                    s.stop.token.cancel();
                }
            }
        }
        self.supply_partial_run_values(fake_node, &mut s);

        // TODO: is this the best way to do this
        match Arc::get_mut(&mut self.ctx_svcs.set.extensions) {
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

                if let ControlFlow::Break(result) =
                    self.collect_and_execute_instructions(&mut s, txs).await
                {
                    match result {
                        Ok(ins) => {
                            s.result.instructions = Some(ins);
                        }
                        Err(error) => {
                            s.flow_error(error);
                        }
                    }
                    // all currently running nodes are in "waiting" State
                    // in `collect_and_execute_instructions`, we sent an error response
                    // so we can wait for them to stop manually
                    // s.stop.token.cancel();
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

        self.collect_flow_output(&mut s).await;

        let failed = s
            .result
            .node_errors
            .iter()
            .filter(|(_, e)| !e.is_empty())
            .count();
        if failed > 0 && !s.early_return {
            s.flow_error(format!("{failed} nodes failed"));
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

        for n in self.nodes.values_mut() {
            n.command.destroy().await;
        }

        s.event_tx.close_channel();

        s.result
    }

    async fn collect_flow_output(&self, s: &mut State) {
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
        if let Some(ins) = s.result.instructions.clone() {
            let fee_payer = ins.fee_payer;
            if !s.result.output.contains_key("transaction") {
                match ins
                    .build_for_solana_action(
                        fee_payer,
                        self.action_identity,
                        &self.ctx_svcs.set.solana_client,
                        self.ctx_svcs.set.helius.as_deref(),
                        self.ctx_data.set.solana.cluster,
                        self.ctx_svcs.signer.clone(),
                        Some(s.flow_run_id),
                        &self.tx_exec_config,
                    )
                    .await
                {
                    Ok(tx) => {
                        let tx_bytes = tx.0.serialize();
                        let tx_base64 = BASE64_STANDARD.encode(&tx_bytes);
                        s.result
                            .output
                            .insert("transaction".into(), Value::String(tx_base64));
                    }
                    Err(error) => {
                        s.flow_error(error.to_string());
                    }
                }
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
                passthrough: passthrough_outputs(&*node.command, &inputs),
                waiting: None,
                instruction_sent: false,
                keypair_outputs: keypair_outputs(&*node.command),
            },
        );
        let outputs = s.result.node_outputs.entry(node.id).or_default();
        debug_assert_eq!(outputs.len(), times as usize);
        outputs.push(<_>::default());
        let rhai_permit = self.rhai_permit.clone();
        let is_rhai_script = rhai_script::is_rhai_script(&node.command.name());
        let span =
            tracing::error_span!(NODE_SPAN_NAME, node_id = node.id.to_string(), times = times);
        let task = run_command()
            .node(node)
            .flow_run_id(s.flow_run_id)
            .times(times)
            .inputs(inputs)
            .ctx_data(self.ctx_data.clone())
            .ctx_svcs(self.ctx_svcs.clone())
            .event_tx(s.event_tx.clone())
            .stop(s.stop.clone())
            .stop_shared(s.stop_shared.clone())
            .tx(s.out_tx.clone())
            .mode(self.mode.clone())
            .tx_exec_config(self.tx_exec_config.clone())
            .get_jwt(self.get_jwt.clone())
            .call();
        let handler = tokio::task::spawn_local(
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

#[derive(Clone)]
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
                output: Ok((Some(req.instructions), req.output)),
                resp: tx,
            })
            .ok();
        rx.map(|r| r?).boxed()
    }
}

#[derive(Clone)]
struct ExecuteNoBundling {
    node_id: NodeId,
    times: u32,
    tx: mpsc::UnboundedSender<PartialOutput>,
    stop_shared: StopSignal,
    simple_svc: execute::Svc,
    overwrite_feepayer: Option<Wallet>,
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
        let overwrite_feepayer = self.overwrite_feepayer.clone();
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
                    output: output.map(|(ins, output)| (Some(ins), output)),
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

struct Finished {
    node: Node,
    times: u32,
    finished_at: DateTime<Utc>,
    result: Result<value::Map, CommandError>,
}

#[allow(clippy::too_many_arguments)]
#[bon::builder]
async fn run_command(
    node: Node,
    flow_run_id: FlowRunId,
    times: u32,
    inputs: value::Map,
    ctx_data: FlowContextData,
    ctx_svcs: FlowServices,
    mut get_jwt: get_jwt::Svc,
    event_tx: EventSender,
    stop: StopSignal,
    stop_shared: StopSignal,
    tx: mpsc::UnboundedSender<PartialOutput>,
    mode: client::BundlingMode,
    tx_exec_config: ExecutionConfig,
) -> Finished {
    let execute = match mode {
        client::BundlingMode::Off => TowerClient::new(ExecuteNoBundling {
            node_id: node.id,
            times,
            tx: tx.clone(),
            simple_svc: simple_execute_svc(
                ctx_svcs.set.solana_client.clone(),
                ctx_svcs.set.helius.clone(),
                ctx_data.set.solana.cluster,
                ctx_svcs.signer.clone(),
                Some(flow_run_id),
                tx_exec_config.clone(),
            ),
            stop_shared,
            overwrite_feepayer: tx_exec_config
                .overwrite_feepayer
                .clone()
                .map(|x| x.to_keypair()),
        }),
        client::BundlingMode::Automatic => TowerClient::new(ExecuteWithBundling {
            node_id: node.id,
            times,
            tx: tx.clone(),
        }),
    };
    if !node.command.permissions().user_tokens {
        get_jwt = get_jwt::Svc::new(service_fn(|_| {
            std::future::ready(Err(get_jwt::Error::NotAllowed))
        }));
    }

    let ctx = CommandContext::builder()
        .data(CommandContextData {
            flow: ctx_data,
            node_id: node.id,
            times,
        })
        .execute(execute)
        .get_jwt(get_jwt)
        .flow(ctx_svcs)
        .node_log(NodeLogSender::new(event_tx.clone(), node.id, times))
        .build();

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

    if f.in_edges(nid).count() == 0 {
        return ran;
    }

    let mut has_array_input = false;
    let mut filled = true;
    let mut all_sources_not_finished = true;

    for e in f.in_edges(nid) {
        if let Some(EdgeValue {
            value: Some(_),
            tracker,
        }) = e.weight().values.front()
        {
            has_array_input |= tracker.is_array();
        } else if e.weight().is_required_input {
            let source_finished = finished_recursive(f, s, e.source(), visited);
            all_sources_not_finished &= !source_finished;
            filled = false;
        }
    }

    if filled {
        if has_array_input { false } else { ran }
    } else {
        !all_sources_not_finished
    }
}

fn new_bfs<G, I>(graph: G, entrypoint: I) -> Bfs<G::NodeId, G::Map>
where
    G: GraphRef + Visitable,
    I: IntoIterator<Item = G::NodeId>,
{
    let mut discovered = graph.visit_map();
    let mut stack = VecDeque::new();
    for id in entrypoint.into_iter() {
        discovered.visit(id);
        stack.push_back(id);
    }
    Bfs { stack, discovered }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use flow_lib::config::client::ClientConfig;
    use flow_lib::flow_run_events::event_channel;

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

    #[actix::test]
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

    #[actix::test]
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
