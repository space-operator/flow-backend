use cmds_bun as _;
use cmds_image as _;
use cmds_pdg as _;
use cmds_solana as _;
use cmds_std as _;

fn main() {
    if let Err(error) = flow_rpc::command_side::command_server::main() {
        eprintln!("all-cmds-server exited: {error:#}");
        std::process::exit(1);
    }
}
