//! RPC specification for calling a command on a remote node

use capnp::capability::{FromClientHook, Promise};
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};

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

pub(crate) fn connect_generic_futures_io<
    R: futures::io::AsyncRead + Unpin + 'static,
    W: futures::io::AsyncWrite + Unpin + 'static,
    C: FromClientHook,
>(
    reader: R,
    writer: W,
) -> C {
    let network = Box::new(VatNetwork::new(
        futures::io::BufReader::new(reader),
        futures::io::BufWriter::new(writer),
        Side::Client,
        Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let client: C = rpc_system.bootstrap(Side::Server);
    tokio::task::spawn_local(rpc_system);
    client
}

pub mod command_side;
pub mod flow_side;
