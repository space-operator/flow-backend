pub mod claim_token;
pub mod confirm_auth;
pub mod init_auth;

pub mod upsert_wallet;

pub mod get_flow_output;
pub mod get_signature_request;

pub mod start_flow;
pub mod start_flow_shared;
pub mod start_flow_unverified;
pub mod stop_flow;

pub mod clone_flow;

pub mod submit_signature;

pub mod apikey_info;
pub mod create_apikey;
pub mod delete_apikey;

pub mod kvstore;

pub mod auth_proxy;
pub mod db_push_logs;
pub mod db_rpc;
pub mod ws_auth_proxy;

pub mod data_export;
pub mod data_import;

pub mod get_info;

pub mod deploy_flow;
pub mod start_deployment;

pub mod prelude {
    pub use crate::{db_worker::DBWorker, error::Error, middleware::auth, Config};
    pub use actix_web::{dev::HttpServiceFactory, http::StatusCode, web};
    pub use db::{
        connection::{UserConnection, UserConnectionTrait},
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
