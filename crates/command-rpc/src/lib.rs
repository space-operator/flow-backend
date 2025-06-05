//! RPC specification for calling a command on a remote node

use capnp::capability::Promise;

pub mod client;

pub(crate) mod command_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/command_capnp.rs"));
}

// https://github.com/capnproto/capnproto-rust/pull/564
pub(crate) fn r2p<T, E>(r: Result<T, E>) -> Promise<T, E> {
    match r {
        Ok(t) => Promise::ok(t),
        Err(e) => Promise::err(e),
    }
}

pub mod command_side;
pub mod flow_side;
