use crate::command_side::command_trait;
use async_trait::async_trait;
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
    async fn new(client: command_trait::Client) -> Result<Self, CommandError> {
        let name = client
            .name_request()
            .send()
            .promise
            .await?
            .get()?
            .get_name()?
            .to_string()?;
        let inputs = serde_json::from_slice(
            client
                .inputs_request()
                .send()
                .promise
                .await?
                .get()?
                .get_inputs()?,
        )?;
        let outputs = serde_json::from_slice(
            client
                .outputs_request()
                .send()
                .promise
                .await?
                .get()?
                .get_outputs()?,
        )?;
        let instruction_info = serde_json::from_slice(
            client
                .instruction_info_request()
                .send()
                .promise
                .await?
                .get()?
                .get_info()?,
        )?;
        let permissions = serde_json::from_slice(
            client
                .permissions_request()
                .send()
                .promise
                .await?
                .get()?
                .get_permissions()?,
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
        let mut req = self.client.run_request();
        req.get()
            .set_ctx(capnp_rpc::new_client(CommandContextImpl { context: ctx }));
        req.get().set_inputs(&map_to_bincode(&params)?);
        let resp = req.send().promise.await?;
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
