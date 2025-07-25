//! RPC specification for calling a command on a remote node

use capnp::capability::FromClientHook;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};

pub mod client;

pub(crate) mod command_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/command_capnp.rs"));
}

pub(crate) fn anyhow2capnp(error: anyhow::Error) -> capnp::Error {
    capnp::Error::failed(format!("{error:#}"))
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

pub(crate) mod make_sync;

pub mod command_side;
pub mod flow_side;
pub mod tracing;
