use std::time::Instant;

use capnp::capability::Promise;
use flow_lib::{context::CommandContext, value};

pub use crate::command_capnp::command_context::*;
use crate::r2p;

pub struct CommandContextImpl {
    pub(crate) context: CommandContext,
}

impl CommandContextImpl {
    fn data_impl(&mut self, _: DataParams, mut result: DataResults) -> Result<(), anyhow::Error> {
        let ctx_data = self.context.raw().data;
        let data = value::to_value(ctx_data)?.to_bincode()?;
        result.get().set_data(&data);
        Ok(())
    }
}

impl Server for CommandContextImpl {
    fn data(&mut self, params: DataParams, result: DataResults) -> Promise<(), ::capnp::Error> {
        r2p(self
            .data_impl(params, result)
            .map_err(|error| capnp::Error::failed(error.to_string())))
    }
}
