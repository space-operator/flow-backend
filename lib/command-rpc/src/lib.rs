//! RPC specification for calling a command on a remote node

use capnp::capability::FromClientHook;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};
use errors::TypedError;

pub mod client;

pub(crate) mod command_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/command_capnp.rs"));
}

pub(crate) fn anyhow2capnp(error: anyhow::Error) -> capnp::Error {
    match TypedError::from(error) {
        TypedError::Capnp(error) => error,
        error => capnp::Error::failed(match serde_json::to_string(&error) {
            Ok(json) => json,
            Err(error) => return capnp::Error::failed(error.to_string()),
        }),
    }
}

pub(crate) fn capnp2typed(error: capnp::Error) -> TypedError {
    if error.kind == capnp::ErrorKind::Failed {
        let extra = error.extra.as_str();
        let extra = extra.strip_prefix("remote exception: ").unwrap_or(extra);
        match serde_json::from_str::<TypedError>(extra) {
            Ok(typed) => typed,
            Err(_) => TypedError::Unknown(anyhow::Error::msg(extra.to_owned())),
        }
    } else {
        TypedError::Capnp(error)
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

pub(crate) mod make_sync;

pub mod command_side;
pub mod errors;
pub mod flow_side;
pub mod tracing;

#[cfg(test)]
pub mod add;
#[cfg(test)]
pub mod error_node;
