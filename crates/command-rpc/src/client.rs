//! A command proxy which calls remote server to execute the actual command.

use async_trait::async_trait;
use flow_lib::{
    command::prelude::*,
    config::Endpoints,
    context::{execute, CommandContext},
    ContextConfig, FlowRunId, NodeId, User,
};
use std::collections::HashMap;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceProxy {
    pub name: String,
    pub id: String,
    pub base_url: Url,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandContextData {
    pub flow_run_id: FlowRunId,
    pub node_id: NodeId,
    pub times: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContextData {
    pub flow_owner: User,
    pub started_by: User,
    pub cfg: ContextConfig,
    pub environment: HashMap<String, String>,
    pub endpoints: Endpoints,
    pub command: Option<CommandContextData>,
    pub signer: ServiceProxy,
}

impl ContextData {
    pub async fn new(
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

        Ok(Self {
            flow_owner,
            started_by,
            cfg,
            environment,
            endpoints,
            command: command.map(Into::into),
            signer: ServiceProxy {
                name: signer.name,
                id: signer.id,
                base_url: signer.base_url,
            },
        })
    }
}

impl From<CommandContextData> for CommandContext {
    fn from(
        CommandContextData {
            flow_run_id,
            node_id,
            times,
        }: CommandContextData,
    ) -> Self {
        Self {
            svc: execute::unimplemented_svc(),
            flow_run_id,
            node_id,
            times,
        }
    }
}

impl From<ContextData> for Context {
    fn from(
        ContextData {
            flow_owner,
            started_by,
            cfg,
            environment,
            endpoints,
            command,
            signer: _,
        }: ContextData,
    ) -> Self {
        Self {
            flow_owner,
            started_by,
            cfg,
            environment,
            endpoints,
            command: command.map(Into::into),
            ..<_>::default()
        }
    }
}

impl From<CommandContext> for CommandContextData {
    fn from(
        CommandContext {
            svc: _,
            flow_run_id,
            node_id,
            times,
        }: CommandContext,
    ) -> Self {
        Self {
            flow_run_id,
            node_id,
            times,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunInput {
    ctx: ContextData,
    params: ValueSet,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunOutput(pub Result<ValueSet, String>);

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
        let rpc = ctx
            .extensions
            .get::<actix::Addr<srpc::Server>>()
            .ok_or_else(|| CommandError::msg("srpc::Server not available"))?
            .clone();
        let ctx_data = ContextData::new(ctx).await?;
        let signer = ctx_data.signer.clone();
        let resp = async move {
            Result::<_, CommandError>::Ok(
                http.post(url)
                    .json(&srpc::Request {
                        envelope: "".to_owned(),
                        svc_name: RUN_SVC.into(),
                        svc_id: self.svc_id.clone(),
                        input: RunInput {
                            ctx: ctx_data,
                            params,
                        },
                    })
                    .send()
                    .await?
                    .json::<srpc::Response<RunOutput>>()
                    .await?,
            )
        };

        let resp = match resp.await {
            Ok(x) => x.data.0.map_err(CommandError::msg),
            Err(x) => Err(x),
        };

        rpc.do_send(srpc::RemoveService {
            name: signer.name,
            id: signer.id,
        });

        resp
    }
}
