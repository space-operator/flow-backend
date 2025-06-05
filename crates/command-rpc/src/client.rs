//! A command proxy which calls remote server to execute the actual command.

use async_trait::async_trait;
use flow_lib::{
    command::{InstructionInfo, prelude::*},
    context::{self, CommandContext},
};
use schemars::JsonSchema;
use serde_with::{DisplayFromStr, serde_as};
use srpc::GetBaseUrl;
use std::convert::Infallible;
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

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
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

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
struct ContextProxy {
    data: context::CommandContextData,
    signer: ServiceProxy,
    execute: ServiceProxy,
    log: ServiceProxy,
}

impl ContextProxy {
    async fn new(ctx: CommandContext) -> Result<Self, CommandError> {
        let server = ctx
            .get::<actix::Addr<srpc::Server>>()
            .ok_or_else(|| CommandError::msg("srpc::Server not available"))?;
        let our_base_url = server
            .send(GetBaseUrl)
            .await?
            .ok_or_else(|| CommandError::msg("srpc::Server is not listening on any interfaces"))?;
        let flow_run_id = *ctx.flow_run_id();

        let signer = ctx.raw().services.signer.clone();

        let signer = server
            .send(srpc::RegisterJsonService::new(
                "signer".to_owned(),
                flow_run_id.to_string(),
                signer,
            ))
            .await?;

        let data = ctx.raw().data.clone();

        let span = tracing::Span::current();
        let id = format!("{};{};{}", flow_run_id, ctx.node_id(), ctx.times());
        let execute = ctx.raw().services.execute.clone();
        let execute = server
            .send(srpc::RegisterJsonService::new(
                "execute".to_owned(),
                id.clone(),
                execute.map_future({
                    let span = span.clone();
                    move |f| f.instrument(span.clone())
                }),
            ))
            .await?;
        let log = server
            .send(srpc::RegisterJsonService::new(
                "log".to_owned(),
                id,
                LogSvc { span },
            ))
            .await?;

        Ok(Self {
            data,
            signer: ServiceProxy::new(signer, our_base_url.clone(), server),
            execute: ServiceProxy::new(execute, our_base_url.clone(), server),
            log: ServiceProxy::new(log, our_base_url, server),
        })
    }
}

#[derive(Serialize, Debug, JsonSchema)]
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

#[async_trait(?Send)]
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

    async fn run(&self, ctx: CommandContext, params: ValueSet) -> Result<ValueSet, CommandError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_schema() {
        let s = schemars::schema_for!(ContextProxy);
        println!("{}", serde_json::to_string_pretty(&s).unwrap());
    }
}
