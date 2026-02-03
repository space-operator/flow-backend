use cmds_deno as _;

fn main() {
    flow_rpc::command_side::command_server::main().unwrap();
}
