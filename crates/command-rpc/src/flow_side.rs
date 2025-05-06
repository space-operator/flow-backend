use crate::command_capnp::command_context;
use capnp::capability::Promise;
use capnp_rpc::pry;
use flow_lib::context::CommandContext;

struct CommandContextImpl {
    context: CommandContext,
}

impl command_context::Server for CommandContextImpl {
    fn data(
        &mut self,
        _: command_context::DataParams,
        mut result: command_context::DataResults,
    ) -> Promise<(), ::capnp::Error> {
        let data = self.context.raw().data;
        result.get().set_data(todo!());
        Promise::err(::capnp::Error::unimplemented(
            "method command_context::Server::data not implemented".to_string(),
        ))
    }
}
