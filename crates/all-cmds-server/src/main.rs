use cmds_pdg as _;
use cmds_solana as _;
use cmds_std as _;

fn main() {
    flow_rpc::command_side::command_server::main().unwrap();
}
