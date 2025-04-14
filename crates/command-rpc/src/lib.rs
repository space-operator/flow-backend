//! RPC specification for calling a command on a remote node

pub mod client;
pub mod rpc_command;
pub mod server;

pub mod command_capnp {
    include!(concat!(env!("OUT_DIR"), "/command_capnp.rs"));
}
