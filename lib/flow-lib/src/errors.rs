use serde_with::serde_conv;

pub use solana_rpc_client_api::client_error::Error as ClientError;

serde_conv!(AsClientError, ClientError, || {}, || {})
