//! A command proxy which calls remote server to execute the actual command.

use async_trait::async_trait;
use flow_lib::{
    ContextConfig, FlowRunId, NodeId, User,
    command::{InstructionInfo, prelude::*},
    config::Endpoints,
    context::CommandContext,
};
use serde_with::{DisplayFromStr, serde_as};
use srpc::GetBaseUrl;
use std::{collections::HashMap, convert::Infallible};
use tower::util::ServiceExt;
use tracing::Instrument;
use url::Url;

struct LogSvc {
    span: tracing::Span,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
struct Log {
    #[serde_as(as = "DisplayFromStr")]
    level: tracing::Level,
    content: String,
}

impl tower::Service<Log> for LogSvc {
    type Error = Infallible;
    type Response = ();
    type Future = std::future::Ready<Result<(), Infallible>>;
    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Log) -> Self::Future {
        self.span.in_scope(|| {
            match req.level {
                tracing::Level::ERROR => tracing::error!(message = req.content),
                tracing::Level::WARN => tracing::warn!(message = req.content),
                tracing::Level::INFO => tracing::info!(message = req.content),
                tracing::Level::DEBUG => tracing::debug!(message = req.content),
                tracing::Level::TRACE => tracing::trace!(message = req.content),
            }
            std::future::ready(Ok(()))
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ServiceProxy {
    name: String,
    id: String,
    base_url: Url,
    #[serde(skip)]
    drop: Option<actix::Addr<srpc::Server>>,
}

impl Drop for ServiceProxy {
    fn drop(&mut self) {
        if let Some(addr) = &self.drop {
            addr.do_send(srpc::RemoveService {
                name: self.name.clone(),
                id: self.id.clone(),
            });
        }
    }
}

impl ServiceProxy {
    fn new(
        result: srpc::RegisterServiceResult,
        base_url: Url,
        server: &actix::Addr<srpc::Server>,
    ) -> Self {
        Self {
            name: result.name,
            id: result.id,
            base_url,
            drop: result.old_service.is_none().then(|| server.clone()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CommandContextData {
    flow_run_id: FlowRunId,
    node_id: NodeId,
    times: u32,
    svc: ServiceProxy,
    log: ServiceProxy,
}

#[derive(Serialize, Deserialize, Debug)]
struct ContextProxy {
    flow_owner: User,
    started_by: User,
    cfg: ContextConfig,
    environment: HashMap<String, String>,
    endpoints: Endpoints,
    command: Option<CommandContextData>,
    signer: ServiceProxy,
}

impl ContextProxy {
    async fn new(
        Context {
            flow_owner,
            started_by,
            cfg,
            http: _,
            solana_client: _,
            environment,
            endpoints,
            extensions,
            command,
            signer,
            get_jwt: _,
        }: Context,
    ) -> Result<Self, CommandError> {
        let server = extensions
            .get::<actix::Addr<srpc::Server>>()
            .ok_or_else(|| CommandError::msg("srpc::Server not available"))?;
        let our_base_url = server
            .send(GetBaseUrl)
            .await?
            .ok_or_else(|| CommandError::msg("srpc::Server is not listening on any interfaces"))?;
        let flow_run_id = command
            .as_ref()
            .map(|c| c.flow_run_id)
            .ok_or_else(|| CommandError::msg("CommandContext not available"))?;

        let signer = server
            .send(srpc::RegisterJsonService::new(
                "signer".to_owned(),
                flow_run_id.to_string(),
                signer,
            ))
            .await?;

        let command = match command {
            Some(command) => {
                Some(CommandContextData::new(command, our_base_url.clone(), server).await?)
            }
            None => None,
        };

        Ok(Self {
            flow_owner,
            started_by,
            cfg,
            environment,
            endpoints,
            command,
            signer: ServiceProxy::new(signer, our_base_url, server),
        })
    }
}

impl CommandContextData {
    async fn new(
        CommandContext {
            svc,
            flow_run_id,
            node_id,
            times,
        }: CommandContext,
        base_url: Url,
        server: &actix::Addr<srpc::Server>,
    ) -> Result<Self, CommandError> {
        let span = tracing::Span::current();
        let svc = server
            .send(srpc::RegisterJsonService::new(
                "execute".to_owned(),
                format!("{};{};{}", flow_run_id, node_id, times),
                svc.map_future({
                    let span = span.clone();
                    move |f| f.instrument(span.clone())
                }),
            ))
            .await?;
        let log = server
            .send(srpc::RegisterJsonService::new(
                "log".to_owned(),
                format!("{};{};{}", flow_run_id, node_id, times),
                LogSvc { span },
            ))
            .await?;
        Ok(Self {
            flow_run_id,
            node_id,
            times,
            svc: ServiceProxy::new(svc, base_url.clone(), server),
            log: ServiceProxy::new(log, base_url, server),
        })
    }
}

#[derive(Serialize, Debug)]
struct RunInput<'a> {
    ctx: &'a ContextProxy,
    params: ValueSet,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RunOutput(Result<ValueSet, String>);

pub struct RpcCommandClient {
    base_url: Url,
    svc_id: String,
    node_data: NodeData,
}

impl RpcCommandClient {
    pub fn new(base_url: Url, svc_id: String, node_data: NodeData) -> Self {
        Self {
            base_url,
            svc_id,
            node_data,
        }
    }
}

const RUN_SVC: &str = "run";

#[async_trait]
impl CommandTrait for RpcCommandClient {
    fn name(&self) -> Name {
        self.node_data.node_id.clone()
    }

    fn inputs(&self) -> Vec<Input> {
        self.node_data.inputs()
    }

    fn outputs(&self) -> Vec<Output> {
        self.node_data.outputs()
    }

    async fn run(&self, ctx: CommandContextX, params: ValueSet) -> Result<ValueSet, CommandError> {
        let url = self.base_url.join("call").unwrap();
        let http = ctx.http().clone();
        let ctx_proxy = ContextProxy::new(ctx).await?;
        let resp = http
            .post(url)
            .json(&srpc::Request {
                // HTTP protocol doesn't need an envelope
                envelope: "".to_owned(),
                svc_name: RUN_SVC.into(),
                svc_id: self.svc_id.clone(),
                input: RunInput {
                    ctx: &ctx_proxy,
                    params,
                },
            })
            .send()
            .await?
            .json::<srpc::Response<RunOutput>>()
            .await?
            .data
            .0
            .map_err(CommandError::msg);

        // ctx_proxy must persist for the duration of the HTTP request
        // although rust won't drop it early
        // we call drop here to make the intention explicit
        drop(ctx_proxy);

        resp
    }

    fn instruction_info(&self) -> Option<InstructionInfo> {
        self.node_data.instruction_info.clone()
    }
}
