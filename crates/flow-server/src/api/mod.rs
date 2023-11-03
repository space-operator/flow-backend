pub mod claim_token;
pub mod confirm_auth;
pub mod init_auth;

pub mod start_flow;
pub mod stop_flow;

pub mod clone_flow;

pub mod submit_signature;

pub mod apikey_info;
pub mod create_apikey;
pub mod delete_apikey;

pub mod kvstore;

pub mod auth_proxy;
pub mod db_rpc;
pub mod db_push_logs;
pub mod ws_auth_proxy;

pub mod prelude {
    pub use crate::{db_worker::DBWorker, error::Error, middleware::auth, Config};
    pub use actix_web::{dev::HttpServiceFactory, http::StatusCode, web};
    pub use db::{
        connection::UserConnection,
        pool::{DbPool, RealDbPool},
        Error as DbError,
    };
    pub use flow_lib::{FlowId, FlowRunId, UserId, ValueSet};
    pub use serde::{Deserialize, Serialize};
    pub use thiserror::Error as ThisError;

    pub struct Success;

    impl Serialize for Success {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut s = s.serialize_struct("Success", 1)?;
            s.serialize_field("success", &true)?;
            s.end()
        }
    }
}

pub mod serde_bs58 {
    pub use super::bs58_decode as deserialize;
    pub use super::bs58_encode as serialize;
}

pub fn bs58_encode<const N: usize, S>(t: &[u8; N], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&bs58::encode(t).into_string())
}

pub fn bs58_decode<'de, const S: usize, D>(d: D) -> Result<[u8; S], D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor<const N: usize>;

    impl<'de, const N: usize> serde::de::Visitor<'de> for Visitor<N> {
        type Value = [u8; N];

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("base58 public key")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut pk = [0u8; N];
            let size = bs58::decode(v)
                .into(&mut pk)
                .map_err(|_| serde::de::Error::custom("invalid base58"))?;
            if size != N {
                return Err(serde::de::Error::custom("invalid base58"));
            }
            Ok(pk)
        }
    }

    d.deserialize_str(Visitor::<S>)
}
