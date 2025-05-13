use crate::command_capnp::{command_context, command_factory, command_trait};
use capnp::capability::Promise;
use flow_lib::{
    CmdInputDescription, CmdOutputDescription, CommandType, Name,
    command::{CommandError, CommandTrait},
    config::client::NodeData,
    context::CommandContext,
    value,
};
use std::{
    collections::BTreeSet,
    future::ready,
    sync::{Arc, Mutex},
};
use thiserror::Error as ThisError;
use tokio::task::LocalSet;

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
                let cmd = resp.get()?.to_owned().get_cmd()?;
            }
        }
        Err(CommandError::msg("not available"))
    }
}

pub struct RemoteCommand {}

impl CommandTrait for RemoteCommand {
    fn name(&self) -> Name {
        todo!()
    }

    fn inputs(&self) -> Vec<CmdInputDescription> {
        todo!()
    }

    fn outputs(&self) -> Vec<CmdOutputDescription> {
        todo!()
    }

    fn run<'life0, 'async_trait>(
        &'life0 self,
        ctx: CommandContext,
        params: value::Map,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<value::Map, CommandError>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }
}
