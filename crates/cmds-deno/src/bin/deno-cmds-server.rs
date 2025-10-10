use cmds_deno as _;

fn main() {
    command_rpc::command_side::command_server::main().unwrap();
}
