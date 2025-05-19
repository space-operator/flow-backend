use crate::command_capnp::{command_context, command_factory, command_trait};
use capnp::capability::Promise;
use flow_lib::{
    CommandType,
    command::prelude::*,
    value::bincode_impl::{map_from_bincode, map_to_bincode},
};
use std::future::ready;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
enum Error {
    #[error(transparent)]
    Value(#[from] value::Error),
    #[error(transparent)]
    BincodeEncode(#[from] bincode::error::EncodeError),
}

impl From<Error> for capnp::Error {
    fn from(value: Error) -> capnp::Error {
        capnp::Error::failed(value.to_string())
    }
}

pub struct CommandContextImpl {
    pub(crate) context: CommandContext,
}

impl CommandContextImpl {
    fn data_impl(
        &mut self,
        _: command_context::DataParams,
        mut result: command_context::DataResults,
    ) -> Result<(), Error> {
        let ctx_data = self.context.raw().data;
        let data = value::to_value(ctx_data)?.to_bincode()?;
        result.get().set_data(&data);
        Ok(())
    }
}

impl command_context::Server for CommandContextImpl {
    fn data(
        &mut self,
        params: command_context::DataParams,
        result: command_context::DataResults,
    ) -> Promise<(), ::capnp::Error> {
        let result = self.data_impl(params, result).map_err(capnp::Error::from);
        Promise::from_future(ready(result))
    }
}

pub struct AddressBook {
    addresses: Vec<Address>,
}

pub struct Address {
    client: command_factory::Client,
    availables: Vec<Available>,
}

struct Available {
    kind: CommandType,
    name: String,
    // version: semver::Version,
}

impl AddressBook {
    pub async fn new_command(
        &self,
        name: &str,
        nd: &NodeData,
    ) -> Result<Box<dyn CommandTrait>, CommandError> {
        for a in &self.addresses {
            if a.availables.iter().any(
                |a| a.kind == nd.r#type && a.name == name, // TODO check version
            ) {
                let mut req = a.client.init_request();
                req.get().set_name(name);
                req.get().set_nd(&simd_json::to_vec(nd)?);
                let resp = req.send().promise.await?;
                let client = resp.get()?.to_owned().get_cmd()?;
                return Ok(Box::new(RemoteCommand::new(client).await?));
            }
        }
        Err(CommandError::msg("not available"))
    }
}

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
        Ok(map_from_bincode(
            req.send().promise.await?.get()?.get_output()?,
        )?)
    }

    fn instruction_info(&self) -> Option<InstructionInfo> {
        self.instruction_info.clone()
    }

    fn permissions(&self) -> Permissions {
        self.permissions.clone()
    }
}
