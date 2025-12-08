use crate::{command_side::command_trait, errors::TypedError};
use async_trait::async_trait;
use capnp::ErrorKind;
use flow_lib::{
    CmdInputDescription, CmdOutputDescription, Name,
    command::{CommandError, CommandTrait, InstructionInfo},
    config::node::Permissions,
    context::CommandContext,
    value::{
        self,
        bincode_impl::{map_from_bincode, map_to_bincode},
    },
};

use super::command_context::CommandContextImpl;

pub struct RemoteCommand {
    client: command_trait::Client,
    name: String,
    inputs: Vec<CmdInputDescription>,
    outputs: Vec<CmdOutputDescription>,
    instruction_info: Option<InstructionInfo>,
    permissions: Permissions,
}

impl RemoteCommand {
    pub async fn new(client: command_trait::Client) -> Result<Self, CommandError> {
        let (name, inputs, outputs, instruction_info, permissions) = tokio::try_join!(
            get_name(&client),
            get_inputs(&client),
            get_outputs(&client),
            get_instruction_info(&client),
            get_permissions(&client)
        )?;
        Ok(RemoteCommand {
            client,
            name,
            inputs,
            outputs,
            instruction_info,
            permissions,
        })
    }
}

async fn get_permissions(client: &command_trait::Client) -> Result<Permissions, anyhow::Error> {
    let permissions = serde_json::from_slice(
        client
            .permissions_request()
            .send()
            .promise
            .await?
            .get()?
            .get_permissions()?,
    )?;
    Ok(permissions)
}

async fn get_outputs(
    client: &command_trait::Client,
) -> Result<Vec<CmdOutputDescription>, anyhow::Error> {
    let outputs = serde_json::from_slice(
        client
            .outputs_request()
            .send()
            .promise
            .await?
            .get()?
            .get_outputs()?,
    )?;
    Ok(outputs)
}

async fn get_instruction_info(
    client: &command_trait::Client,
) -> Result<Option<InstructionInfo>, anyhow::Error> {
    let instruction_info = serde_json::from_slice(
        client
            .instruction_info_request()
            .send()
            .promise
            .await?
            .get()?
            .get_info()?,
    )?;
    Ok(instruction_info)
}

async fn get_inputs(
    client: &command_trait::Client,
) -> Result<Vec<CmdInputDescription>, anyhow::Error> {
    let inputs = serde_json::from_slice(
        client
            .inputs_request()
            .send()
            .promise
            .await?
            .get()?
            .get_inputs()?,
    )?;
    Ok(inputs)
}

async fn get_name(client: &command_trait::Client) -> Result<String, anyhow::Error> {
    let name = client
        .name_request()
        .send()
        .promise
        .await?
        .get()?
        .get_name()?
        .to_string()?;
    Ok(name)
}

#[async_trait(?Send)]
impl CommandTrait for RemoteCommand {
    fn name(&self) -> Name {
        self.name.clone()
    }

    fn inputs(&self) -> Vec<CmdInputDescription> {
        self.inputs.clone()
    }

    fn outputs(&self) -> Vec<CmdOutputDescription> {
        self.outputs.clone()
    }

    async fn run(
        &self,
        ctx: CommandContext,
        params: value::Map,
    ) -> Result<value::Map, CommandError> {
        let ctx_client = capnp_rpc::new_client(CommandContextImpl { context: ctx });
        let mut req = self.client.run_request();
        req.get().set_ctx(ctx_client);
        req.get().set_inputs(&map_to_bincode(&params)?);
        let resp = match req.send().promise.await {
            Ok(resp) => resp,
            Err(error) => {
                return if error.kind == ErrorKind::Failed {
                    let extra = error.extra.as_str();
                    let extra = extra.strip_prefix("remote exception: ").unwrap_or(&extra);
                    match serde_json::from_str::<TypedError>(extra) {
                        Ok(typed) => Err(typed.to_anyhow()),
                        Err(_) => Err(CommandError::msg(extra.to_owned())),
                    }
                } else {
                    Err(error.into())
                };
            }
        };
        let outputs = resp.get()?.get_output()?;
        Ok(map_from_bincode(outputs)?)
    }

    fn instruction_info(&self) -> Option<InstructionInfo> {
        self.instruction_info.clone()
    }

    fn permissions(&self) -> Permissions {
        self.permissions.clone()
    }
}
