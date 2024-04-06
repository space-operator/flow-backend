//! A command proxy which calls remote server to execute the actual command.

use async_trait::async_trait;
use flow_lib::{
    command::prelude::*, config::Endpoints, context::CommandContext, ContextConfig, FlowRunId,
    NodeId, User,
};
use std::collections::HashMap;
use url::Url;

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
    fn new(result: srpc::RegisterServiceResult, server: &actix::Addr<srpc::Server>) -> Self {
        Self {
            name: result.name,
            id: result.id,
            base_url: result.base_url,
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
        let flow_run_id = command
            .as_ref()
            .map(|c| c.flow_run_id)
            .ok_or_else(|| CommandError::msg("CommandContext not available"))?;

        let signer = server
            .send(srpc::RegisterJsonService::new(
                "signer".to_string(),
                flow_run_id.to_string(),
                signer,
            ))
            .await?;

        let command = match command {
            Some(command) => Some(CommandContextData::new(command, server).await?),
            None => None,
        };

        Ok(Self {
            flow_owner,
            started_by,
            cfg,
            environment,
            endpoints,
            command,
            signer: ServiceProxy::new(signer, server),
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
        server: &actix::Addr<srpc::Server>,
    ) -> Result<Self, CommandError> {
        let svc = server
            .send(srpc::RegisterJsonService::new(
                "execute".to_string(),
                format!("{}::{}::{}", flow_run_id, node_id, times),
                svc,
            ))
            .await?;
        Ok(Self {
            flow_run_id,
            node_id,
            times,
            svc: ServiceProxy::new(svc, server),
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

    async fn run(&self, ctx: Context, params: ValueSet) -> Result<ValueSet, CommandError> {
        let url = self.base_url.join("call").unwrap();
        let http = ctx.http.clone();
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
}
