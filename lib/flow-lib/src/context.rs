//! Providing services and information for nodes to use.
//!
//! Services are abstracted with [`tower::Service`] trait, using our
//! [`TowerClient`][crate::utils::TowerClient] utility to make it easier to use.
//!
//! Each service is defined is a separated module:
//! - [`get_jwt`]
//! - [`execute`]
//! - [`signer`]

use crate::{
    ContextConfig, FlowRunId, NodeId, SolanaClientConfig, UserId,
    config::{Endpoints, client::FlowRunOrigin},
    solana::Instructions,
    utils::{Extensions, tower_client::unimplemented_svc},
};
use bytes::Bytes;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient as SolanaClient;
use std::{any::Any, collections::HashMap, sync::Arc, time::Duration};
use tower::{Service, ServiceExt};

pub mod env {
    pub const RUST_LOG: &str = "RUST_LOG";
    pub const OVERWRITE_FEEPAYER: &str = "OVERWRITE_FEEPAYER";
    pub const COMPUTE_BUDGET: &str = "COMPUTE_BUDGET";
    pub const FALLBACK_COMPUTE_BUDGET: &str = "FALLBACK_COMPUTE_BUDGET";
    pub const PRIORITY_FEE: &str = "PRIORITY_FEE";
    pub const SIMULATION_COMMITMENT_LEVEL: &str = "SIMULATION_COMMITMENT_LEVEL";
    pub const TX_COMMITMENT_LEVEL: &str = "TX_COMMITMENT_LEVEL";
    pub const WAIT_COMMITMENT_LEVEL: &str = "WAIT_COMMITMENT_LEVEL";
    pub const EXECUTE_ON: &str = "EXECUTE_ON";
    pub const DEVNET_LOOKUP_TABLE: &str = "DEVNET_LOOKUP_TABLE";
    pub const MAINNET_LOOKUP_TABLE: &str = "MAINNET_LOOKUP_TABLE";
}

/// Get user's JWT, require
/// [`user_token`][crate::config::node::Permissions::user_tokens] permission.
pub mod get_jwt {
    use crate::{UserId, utils::TowerClient, utils::tower_client::CommonError};
    use std::future::Ready;
    use thiserror::Error as ThisError;

    #[derive(Clone, Copy)]
    pub struct Request {
        pub user_id: UserId,
    }

    #[derive(Clone, Debug)]
    pub struct Response {
        pub access_token: String,
    }

    #[derive(ThisError, Debug, Clone)]
    pub enum Error {
        #[error("not allowed")]
        NotAllowed,
        #[error("user not found")]
        UserNotFound,
        #[error("wrong recipient")]
        WrongRecipient,
        #[error("{}: {}", error, error_description)]
        Supabase {
            error: String,
            error_description: String,
        },
        #[error(transparent)]
        Common(#[from] CommonError),
    }

    impl actix::Message for Request {
        type Result = Result<Response, Error>;
    }

    pub type Svc = TowerClient<Request, Response, Error>;

    pub fn not_allowed() -> Svc {
        Svc::new(tower::service_fn(|_| {
            std::future::ready(Result::<Response, _>::Err(Error::NotAllowed))
        }))
    }

    #[derive(Clone, Copy, Debug)]
    pub struct RetryPolicy(pub usize);

    impl Default for RetryPolicy {
        fn default() -> Self {
            Self(1)
        }
    }

    impl tower::retry::Policy<Request, Response, Error> for RetryPolicy {
        type Future = Ready<()>;

        fn retry(
            &mut self,
            _: &mut Request,
            result: &mut Result<Response, Error>,
        ) -> Option<Self::Future> {
            match result {
                Err(Error::Supabase {
                    error_description, ..
                }) if error_description.contains("Refresh Token") && self.0 > 0 => {
                    tracing::error!("get_jwt error: {}, retrying", error_description);
                    self.0 -= 1;
                    Some(std::future::ready(()))
                }
                _ => None,
            }
        }

        fn clone_request(&mut self, req: &Request) -> Option<Request> {
            Some(*req)
        }
    }
}

/// Request Solana signature from external wallets.
pub mod signer {
    use crate::{
        FlowRunId,
        solana::{Pubkey, SdkPresigner, Signature},
        utils::{TowerClient, tower_client::CommonError},
    };
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use serde_with::{DisplayFromStr, DurationSecondsWithFrac, base64::Base64, serde_as};
    use std::time::Duration;
    use thiserror::Error as ThisError;

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error("can't sign for pubkey: {}", .0)]
        Pubkey(String),
        #[error("can't sign for this user")]
        User,
        #[error("timeout")]
        Timeout,
        #[error(transparent)]
        Common(#[from] CommonError),
    }

    pub type Svc = TowerClient<SignatureRequest, SignatureResponse, Error>;

    #[serde_as]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Presigner {
        #[serde_as(as = "DisplayFromStr")]
        pub pubkey: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        pub signature: Signature,
    }

    impl From<Presigner> for SdkPresigner {
        fn from(value: Presigner) -> Self {
            SdkPresigner::new(&value.pubkey, &value.signature)
        }
    }

    #[serde_as]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SignatureRequest {
        pub id: Option<i64>,
        #[serde(with = "chrono::serde::ts_milliseconds")]
        pub time: DateTime<Utc>,
        #[serde_as(as = "DisplayFromStr")]
        pub pubkey: Pubkey,
        #[serde_as(as = "Base64")]
        pub message: bytes::Bytes,
        #[serde_as(as = "DurationSecondsWithFrac<f64>")]
        pub timeout: Duration,
        pub flow_run_id: Option<FlowRunId>,
        pub signatures: Option<Vec<Presigner>>,
    }

    impl actix::Message for SignatureRequest {
        type Result = Result<SignatureResponse, Error>;
    }

    #[serde_as]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SignatureResponse {
        #[serde_as(as = "DisplayFromStr")]
        pub signature: Signature,
        #[serde_as(as = "Option<Base64>")]
        pub new_message: Option<bytes::Bytes>,
    }
}

/// Output values and Solana instructions to be executed.
pub mod execute {
    use crate::{
        FlowRunId, SolanaNet,
        solana::{ExecutionConfig, Instructions},
        utils::{
            TowerClient,
            tower_client::{CommonError, CommonErrorExt},
        },
    };
    use futures::channel::oneshot::Canceled;
    use serde::{Deserialize, Serialize};
    use serde_with::{DisplayFromStr, base64::Base64, serde_as};
    use solana_program::{
        instruction::InstructionError, message::CompileError, sanitize::SanitizeError,
    };
    use solana_rpc_client::nonblocking::rpc_client::RpcClient as SolanaClient;
    use solana_rpc_client_api::client_error::Error as ClientError;
    use solana_signature::Signature;
    use solana_signer::SignerError;
    use std::sync::Arc;
    use thiserror::Error as ThisError;

    use super::signer;

    pub type Svc = TowerClient<Request, Response, Error>;

    #[derive(Deserialize)]
    #[serde(try_from = "RequestRepr")]
    pub struct Request {
        pub instructions: Instructions,
        pub output: value::Map,
    }

    #[serde_as]
    #[derive(Deserialize)]
    struct RequestRepr {
        #[serde_as(as = "Base64")]
        instructions: Vec<u8>,
        output: value::Map,
    }

    impl TryFrom<RequestRepr> for Request {
        type Error = rmp_serde::decode::Error;
        fn try_from(value: RequestRepr) -> Result<Self, Self::Error> {
            Ok(Self {
                instructions: rmp_serde::from_slice(&value.instructions)?,
                output: value.output,
            })
        }
    }

    #[serde_as]
    #[derive(Serialize, Clone, Copy)]
    pub struct Response {
        #[serde_as(as = "Option<DisplayFromStr>")]
        pub signature: Option<Signature>,
    }

    fn unwrap(s: &Option<String>) -> &str {
        s.as_ref().map(|v| v.as_str()).unwrap_or_default()
    }

    #[derive(ThisError, Debug, Clone)]
    pub enum Error {
        #[error("canceled {}", unwrap(.0))]
        Canceled(Option<String>),
        #[error("not available on this Context")]
        NotAvailable,
        #[error("some node failed to provide instructions")]
        TxIncomplete,
        #[error("time out")]
        Timeout,
        #[error("insufficient solana balance, needed={needed}; have={balance};")]
        InsufficientSolanaBalance { needed: u64, balance: u64 },
        #[error("transaction simulation failed")]
        TxSimFailed,
        #[error("{}", crate::solana::verbose_solana_error(.error))]
        Solana {
            #[source]
            error: Arc<ClientError>,
            inserted: usize,
        },
        #[error(transparent)]
        Signer(#[from] Arc<SignerError>),
        #[error(transparent)]
        CompileError(#[from] Arc<CompileError>),
        #[error(transparent)]
        InstructionError(#[from] Arc<InstructionError>),
        #[error(transparent)]
        SanitizeError(#[from] Arc<SanitizeError>),
        #[error(transparent)]
        ChannelClosed(#[from] Canceled),
        #[error(transparent)]
        Common(#[from] CommonError),
    }

    impl Error {
        pub fn solana(error: ClientError, inserted: usize) -> Self {
            Self::Solana {
                error: Arc::new(error),
                inserted,
            }
        }
    }

    impl From<signer::Error> for Error {
        fn from(value: signer::Error) -> Self {
            match value {
                e @ signer::Error::Pubkey(_) => Self::other(e),
                e @ signer::Error::User => Self::other(e),
                signer::Error::Timeout => Self::Timeout,
                signer::Error::Common(error) => Self::Common(error),
            }
        }
    }

    impl From<SignerError> for Error {
        fn from(value: SignerError) -> Self {
            Error::Signer(Arc::new(value))
        }
    }

    impl From<CompileError> for Error {
        fn from(value: CompileError) -> Self {
            Error::CompileError(Arc::new(value))
        }
    }

    impl From<InstructionError> for Error {
        fn from(value: InstructionError) -> Self {
            Error::InstructionError(Arc::new(value))
        }
    }

    impl From<SanitizeError> for Error {
        fn from(value: SanitizeError) -> Self {
            Error::SanitizeError(Arc::new(value))
        }
    }

    pub fn simple(
        rpc: Arc<SolanaClient>,
        network: SolanaNet,
        signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
        config: ExecutionConfig,
    ) -> Svc {
        let handle = move |req: Request| {
            let rpc = rpc.clone();
            let signer = signer.clone();
            let config = config.clone();
            async move {
                Ok(Response {
                    signature: Some(
                        req.instructions
                            .execute(&rpc, network, signer, flow_run_id, config)
                            .await?,
                    ),
                })
            }
        };
        Svc::new(tower::service_fn(handle))
    }
}

#[derive(Clone)]
pub struct CommandContext {
    pub svc: execute::Svc,
    pub flow_run_id: FlowRunId,
    pub node_id: NodeId,
    pub times: u32,
}

pub struct FlowSetContextData {
    flow_owner: User,
    started_by: User,
    endpoints: Endpoints,
    solana: SolanaClientConfig,
    extensions: Arc<Extensions>,
}

struct FlowContextData {
    flow_run_id: FlowRunId,
    environment: HashMap<String, String>,
    set: FlowSetContextData,
}

pub struct CommandContextData {
    node_id: NodeId,
    times: u32,
    flow: FlowContextData,
}

pub struct FlowSetContext {
    http: reqwest::Client,
    solana_client: Arc<SolanaClient>,
}

pub struct FlowContext {
    signer: signer::Svc,
    set: FlowSetContext,
}

pub struct CommandContextX {
    data: CommandContextData,
    execute: execute::Svc,
    get_jwt: get_jwt::Svc,
    flow: FlowContext,
}

impl CommandContextX {
    pub fn test_context() -> Self {
        let config = ContextConfig::default();
        let solana_client = Arc::new(SolanaClient::new_with_timeouts_and_commitment(
            config.solana_client.url.clone(),
            Duration::from_secs(30),
            CommitmentConfig {
                commitment: CommitmentLevel::Finalized,
            },
            Duration::from_secs(180),
        ));
        Self {
            data: CommandContextData {
                node_id: NodeId::nil(),
                times: 0,
                flow: FlowContextData {
                    flow_run_id: FlowRunId::nil(),
                    environment: HashMap::new(),
                    set: FlowSetContextData {
                        flow_owner: User::default(),
                        started_by: User::default(),
                        endpoints: Endpoints::default(),
                        solana: config.solana_client,
                        extensions: <_>::default(),
                    },
                },
            },
            execute: unimplemented_svc(),
            get_jwt: unimplemented_svc(),
            flow: FlowContext {
                signer: unimplemented_svc(),
                set: FlowSetContext {
                    http: reqwest::Client::new(),
                    solana_client,
                },
            },
        }
    }

    pub fn environment(&self) -> &HashMap<String, String> {
        &self.data.flow.environment
    }

    pub fn endpoints(&self) -> &Endpoints {
        &self.data.flow.set.endpoints
    }

    pub fn flow_owner(&self) -> &User {
        &self.data.flow.set.flow_owner
    }

    pub fn started_by(&self) -> &User {
        &self.data.flow.set.started_by
    }

    pub fn solana_config(&self) -> &SolanaClientConfig {
        &self.data.flow.set.solana
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.flow.set.http
    }

    /// Call [`get_jwt`] service, the result will have `Bearer ` prefix.
    pub async fn get_jwt_header(&mut self) -> Result<String, get_jwt::Error> {
        let user_id = self.flow_owner().id;
        let access_token = self
            .get_jwt
            .ready()
            .await?
            .call(get_jwt::Request { user_id })
            .await?
            .access_token;
        Ok(["Bearer ", &access_token].concat())
    }
}

impl Default for CommandContextX {
    fn default() -> Self {
        Self::test_context()
    }
}

#[derive(Clone)]
pub struct Context {
    pub flow_owner: User,
    pub started_by: User,
    pub cfg: ContextConfig,
    pub http: reqwest::Client,
    pub solana_client: Arc<SolanaClient>,
    pub environment: HashMap<String, String>,
    pub endpoints: Endpoints,
    pub extensions: Arc<Extensions>,
    pub command: Option<CommandContext>,
    pub signer: signer::Svc,
    pub get_jwt: get_jwt::Svc,
}

impl Default for Context {
    fn default() -> Self {
        let mut ctx = Context::from_cfg(
            &ContextConfig::default(),
            User::default(),
            User::default(),
            unimplemented_svc(),
            unimplemented_svc(),
            Extensions::default(),
        );
        // ctx.command = Some(CommandContext {
        //     svc: execute::simple(&ctx, None, <_>::default()),
        //     flow_run_id: uuid::Uuid::nil(),
        //     node_id: uuid::Uuid::nil(),
        //     times: 0,
        // });
        ctx
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct User {
    pub id: UserId,
}

impl User {
    pub fn new(id: UserId) -> Self {
        Self { id }
    }
}

impl Default for User {
    /// For testing
    fn default() -> Self {
        User {
            id: uuid::Uuid::nil(),
        }
    }
}

impl Context {
    pub fn from_cfg(
        cfg: &ContextConfig,
        flow_owner: User,
        started_by: User,
        sig_svc: signer::Svc,
        token_svc: get_jwt::Svc,
        extensions: Extensions,
    ) -> Self {
        let solana_client = SolanaClient::new_with_timeouts_and_commitment(
            cfg.solana_client.url.clone(),
            Duration::from_secs(30),
            CommitmentConfig {
                commitment: CommitmentLevel::Finalized,
            },
            Duration::from_secs(180),
        );

        Self {
            flow_owner,
            started_by,
            cfg: cfg.clone(),
            http: reqwest::Client::new(),
            solana_client: Arc::new(solana_client),
            environment: cfg.environment.clone(),
            endpoints: cfg.endpoints.clone(),
            extensions: Arc::new(extensions),
            command: None,
            signer: sig_svc,
            get_jwt: token_svc,
        }
    }

    /// Call [`get_jwt`] service, the result will have `Bearer ` prefix.
    pub async fn get_jwt_header(&mut self) -> Result<String, get_jwt::Error> {
        Ok("Bearer ".to_owned()
            + &self
                .get_jwt
                .ready()
                .await?
                .call(get_jwt::Request {
                    user_id: self.flow_owner.id,
                })
                .await?
                .access_token)
    }

    pub fn new_interflow_origin(&self) -> Option<FlowRunOrigin> {
        let c = self.command.as_ref()?;
        Some(FlowRunOrigin::Interflow {
            flow_run_id: c.flow_run_id,
            node_id: c.node_id,
            times: c.times,
        })
    }

    /// Call [`execute`] service.
    pub async fn execute(
        &mut self,
        instructions: Instructions,
        output: value::Map,
    ) -> Result<execute::Response, execute::Error> {
        if let Some(ctx) = &mut self.command {
            ctx.svc
                .ready()
                .await?
                .call(execute::Request {
                    instructions,
                    output,
                })
                .await
        } else {
            Err(execute::Error::NotAvailable)
        }
    }

    /// Call [`signer`] service.
    pub async fn request_signature(
        &self,
        pubkey: Pubkey,
        message: Bytes,
        timeout: Duration,
    ) -> Result<signer::SignatureResponse, anyhow::Error> {
        let mut s = self.signer.clone();

        Ok(s.ready()
            .await?
            .call(signer::SignatureRequest {
                id: None,
                time: Utc::now(),
                pubkey,
                message,
                timeout,
                flow_run_id: self.command.as_ref().map(|ctx| ctx.flow_run_id),
                signatures: None,
            })
            .await?)
    }

    /// Get an extension by type.
    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get::<T>()
    }

    // A function to make sure Context is Send + Sync,
    // because !Sync will make it really hard to write async code.
    #[allow(dead_code)]
    const fn assert_send_sync() {
        const fn f<T: Send + Sync + 'static>() {}
        f::<Self>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_tokio() {
        Context::default();
    }
}
