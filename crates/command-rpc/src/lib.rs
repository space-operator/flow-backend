//! RPC specification for calling a command on a remote node

pub mod client;
pub mod server;

pub mod command_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/command_capnp.rs"));
}

pub mod command_side;
pub mod flow_side;
