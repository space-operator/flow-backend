use cmds_deno as _;

fn main() {
    if let Err(error) = flow_rpc::command_side::command_server::main() {
        tracing::error!("deno-cmds-server exited: {error:#}");
        std::process::exit(1);
    }
}
