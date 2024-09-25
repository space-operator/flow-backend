//! 2-way communication:
//! - CommandHost: contain code to run some commands.
//! - FlowHost: run flows and commands, but might need to use CommandHost to run some commands.
//!
//! CommandHost will connect first, FlowHost will listen on publicly available interfaces.
//!
//! When starting commands:
//! - FlowHost make a proxy on localhost that forward to CommandHost
//! - RpcCommandClient connect to the localhost proxy
//! - CommandHost start the actual command and interchange data though the proxy
//!
//! Starting a command:
//! - Send a request using RUN_SVC, input type is RunInput
//!     - FlowHost must create proxies for signer, log, execute services.
//! - When the command finished, the server will return RunOutput

use flow_lib::command::CommandDescription;
use std::{borrow::Cow, collections::BTreeMap};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),
}

pub type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub struct CommandHost {
    natives: BTreeMap<Cow<'static, str>, CommandDescription>,
    stream: WsStream,
}

async fn send() {}

impl CommandHost {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let (stream, _) = tokio_tungstenite::connect_async(url).await?;
        let mut natives = BTreeMap::new();
        for d in inventory::iter::<CommandDescription>() {
            let name = d.name.clone();
            if natives.insert(name.clone(), d.clone()).is_some() {
                tracing::error!("duplicated command {:?}", name);
            }
        }
        Ok(Self {
            stream,
            natives: Default::default(),
        })
    }
}

pub struct FlowHost {}
