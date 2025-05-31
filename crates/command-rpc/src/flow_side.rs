use crate::command_capnp::{command_context, command_factory, command_trait};
use bincode::config::standard;
use capnp::capability::Promise;
use flow_lib::{
    CommandType,
    command::prelude::*,
    value::bincode_impl::{map_from_bincode, map_to_bincode},
};
use futures::future::LocalBoxFuture;
use std::{cell::RefCell, rc::Rc};
use thiserror::Error as ThisError;

pub mod address_book;

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
        match result {
            Ok(result) => Promise::ok(result),
            Err(error) => Promise::err(error),
        }
    }
}

pub struct FactoryAddressBook {
    addresses: Rc<RefCell<Vec<Address>>>,
}

impl FactoryAddressBook {
    fn join_impl(
        &mut self,
        params: flow_server::JoinParams,
        _: flow_server::JoinResults,
    ) -> LocalBoxFuture<'static, Result<(), capnp::Error>> {
        let addrs = self.addresses.clone();
        Box::pin(async move {
            let client = params.get()?.get_factory()?;
            let names: Vec<String> = bincode::decode_from_slice(
                client
                    .all_availables_request()
                    .send()
                    .promise
                    .await?
                    .get()?
                    .get_availables()?,
                standard(),
            )
            .map_err(|e| capnp::Error::failed(e.to_string()))?
            .0;
            let availables = names
                .into_iter()
                .map(|name| Available {
                    kind: CommandType::Native, // TODO: all native at the moment
                    name,
                })
                .collect();
            addrs.borrow_mut().push(Address { client, availables });
            Ok(())
        })
    }
}

impl flow_server::Server for FactoryAddressBook {
    fn join(
        &mut self,
        params: flow_server::JoinParams,
        results: flow_server::JoinResults,
    ) -> Promise<(), ::capnp::Error> {
        Promise::from_future(self.join_impl(params, results))
    }
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

impl FactoryAddressBook {
    pub fn new_command(
        &self,
        name: &str,
        nd: &NodeData,
    ) -> Result<LocalBoxFuture<'static, Result<Box<dyn CommandTrait>, CommandError>>, CommandError>
    {
        let mut client = None;
        for a in self.addresses.borrow().iter() {
            if a.availables.iter().any(
                |a| a.kind == nd.r#type && a.name == name, // TODO check version
            ) {
                client = Some(a.client.clone());
                break;
            }
        }
        if let Some(client) = client {
            let mut req = client.init_request();
            req.get().set_name(name);
            req.get().set_nd(&simd_json::to_vec(nd)?);
            Ok(Box::pin(async move {
                let resp = req.send().promise.await?;
                let client = resp.get()?.to_owned().get_cmd()?;
                let boxed: Box<dyn CommandTrait> = Box::new(RemoteCommand::new(client).await?);
                Ok(boxed)
            }))
        } else {
            Err(CommandError::msg("not available"))
        }
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

#[cfg(test)]
mod tests;
