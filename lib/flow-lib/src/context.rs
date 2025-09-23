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
    ContextConfig, FlowRunId, HttpClientConfig, NodeId, SolanaClientConfig, UserId, ValueSet,
    config::{Endpoints, client::FlowRunOrigin},
    flow_run_events::{self, Event, NodeLogContent, NodeLogSender},
    solana::Instructions,
    utils::{Extensions, tower_client::unimplemented_svc},
};
use bytes::Bytes;
use chrono::Utc;
use futures::channel::mpsc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

pub mod api_input {
    use std::time::Duration;

    use crate::{
        FlowRunId, NodeId,
        utils::{TowerClient, tower_client::CommonError},
    };
    use thiserror::Error as ThisError;
    use value::Value;

    pub struct Request {
        pub flow_run_id: FlowRunId,
        pub node_id: NodeId,
        pub times: u32,
        pub timeout: Duration,
    }

    pub struct Response {
        pub value: Value,
    }

    #[derive(ThisError, Debug, Clone)]
    pub enum Error {
        #[error("canceled by user")]
        Canceled,
        #[error("timeout")]
        Timeout,
        #[error(transparent)]
        Common(#[from] CommonError),
    }

    pub type Svc = TowerClient<Request, Response, Error>;
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

    impl From<actix::MailboxError> for Error {
        fn from(value: actix::MailboxError) -> Self {
            CommonError::from(value).into()
        }
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
        utils::{TowerClient, tower_client::CommonError},
    };
    use actix::MailboxError;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use serde_with::{DisplayFromStr, DurationSecondsWithFrac, base64::Base64, serde_as};
    use solana_presigner::Presigner as SdkPresigner;
    use solana_pubkey::Pubkey;
    use solana_signature::Signature;
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

    impl From<MailboxError> for Error {
        fn from(value: MailboxError) -> Self {
            CommonError::from(value).into()
        }
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

    impl bincode::Encode for SignatureResponse {
        fn encode<E: bincode::enc::Encoder>(
            &self,
            encoder: &mut E,
        ) -> Result<(), bincode::error::EncodeError> {
            self.signature.as_array().encode(encoder)?;
            self.new_message
                .as_ref()
                .map(|m| m.as_ref())
                .encode(encoder)?;
            Ok(())
        }
    }

    impl<C> bincode::Decode<C> for SignatureResponse {
        fn decode<D: bincode::de::Decoder<Context = C>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            let signature = Signature::from(<[u8; 64]>::decode(decoder)?);
            let new_message = Option::<Vec<u8>>::decode(decoder)?.map(Into::into);
            Ok(Self {
                signature,
                new_message,
            })
        }
    }

    impl<'de, C> bincode::BorrowDecode<'de, C> for SignatureResponse {
        fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = C>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            bincode::Decode::decode(decoder)
        }
    }
}

/// Output values and Solana instructions to be executed.
pub mod execute {
    use crate::{
        solana::Instructions,
        utils::{
            TowerClient,
            tower_client::{CommonError, CommonErrorExt},
        },
    };
    use futures::channel::oneshot::Canceled;
    use serde::{Deserialize, Serialize};
    use serde_with::{DisplayFromStr, base64::Base64, serde_as, serde_conv};
    use solana_program::{
        instruction::InstructionError, message::CompileError, sanitize::SanitizeError,
    };

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

    impl bincode::Encode for Request {
        fn encode<E: bincode::enc::Encoder>(
            &self,
            encoder: &mut E,
        ) -> Result<(), bincode::error::EncodeError> {
            self.instructions.encode(encoder)?;
            value::bincode_impl::MapBincode::from(&self.output).encode(encoder)?;
            Ok(())
        }
    }

    impl<C> bincode::Decode<C> for Request {
        fn decode<D: bincode::de::Decoder<Context = C>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            Ok(Self {
                instructions: Instructions::decode(decoder)?,
                output: value::bincode_impl::MapBincode::decode(decoder)?
                    .0
                    .into_owned(),
            })
        }
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

    impl bincode::Encode for Response {
        fn encode<E: bincode::enc::Encoder>(
            &self,
            encoder: &mut E,
        ) -> Result<(), bincode::error::EncodeError> {
            self.signature.map(|s| *s.as_array()).encode(encoder)?;
            Ok(())
        }
    }

    impl<C> bincode::Decode<C> for Response {
        fn decode<D: bincode::de::Decoder<Context = C>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            let value = Option::<[u8; 64]>::decode(decoder)?;
            Ok(Self {
                signature: value.map(Signature::from),
            })
        }
    }

    fn unwrap(s: &Option<String>) -> &str {
        s.as_ref().map(|v| v.as_str()).unwrap_or_default()
    }

    #[derive(Serialize, Deserialize)]
    enum AsRpcRequestImpl {
        Custom { method: &'static str },
        DeregisterNode,
        GetAccountInfo,
        GetBalance,
        GetBlock,
        GetBlockHeight,
        GetBlockProduction,
        GetBlocks,
        GetBlocksWithLimit,
        GetBlockTime,
        GetClusterNodes,
        GetEpochInfo,
        GetEpochSchedule,
        GetFeeForMessage,
        GetFirstAvailableBlock,
        GetGenesisHash,
        GetHealth,
        GetIdentity,
        GetInflationGovernor,
        GetInflationRate,
        GetInflationReward,
        GetLargestAccounts,
        GetLatestBlockhash,
        GetLeaderSchedule,
        GetMaxRetransmitSlot,
        GetMaxShredInsertSlot,
        GetMinimumBalanceForRentExemption,
        GetMultipleAccounts,
        GetProgramAccounts,
        GetRecentPerformanceSamples,
        GetRecentPrioritizationFees,
        GetHighestSnapshotSlot,
        GetSignaturesForAddress,
        GetSignatureStatuses,
        GetSlot,
        GetSlotLeader,
        GetSlotLeaders,
        GetStorageTurn,
        GetStorageTurnRate,
        GetSlotsPerSegment,
        GetStakeMinimumDelegation,
        GetStoragePubkeysForSlot,
        GetSupply,
        GetTokenAccountBalance,
        GetTokenAccountsByDelegate,
        GetTokenAccountsByOwner,
        GetTokenLargestAccounts,
        GetTokenSupply,
        GetTransaction,
        GetTransactionCount,
        GetVersion,
        GetVoteAccounts,
        IsBlockhashValid,
        MinimumLedgerSlot,
        RegisterNode,
        RequestAirdrop,
        SendTransaction,
        SimulateTransaction,
        SignVote,
    }

    enum AsErrorKindImpl {
        Io { kind: u32, msg: String },
        // These get turned in to Custom
        // Reqwest(Error),
        // Middleware(Error),
        // SerdeJson(Error),
        RpcError(RpcError),
        SigningError(SignerError),
        TransactionError(TransactionError),
        Custom(String),
    }

    struct AsClientErrorImpl {
        request: Option<AsRpcRequestImpl>,
        kind: Box<AsErrorKindImpl>,
    }

    serde_conv!(AsClientError, ClientError, |e: &ClientError| {}, || {});

    #[serde_as]
    #[derive(ThisError, Debug, Clone, Serialize, Deserialize)]
    pub enum Error {
        #[error("canceled {}", unwrap(.0))]
        Canceled(Option<String>),
        #[error("collected")]
        Collected,
        #[error("some node failed to provide instructions")]
        TxIncomplete,
        #[error("time out")]
        Timeout,
        #[error("insufficient solana balance, needed={needed}; have={balance};")]
        InsufficientSolanaBalance { needed: u64, balance: u64 },
        #[error("transaction simulation failed")]
        TxSimFailed,
        #[error("{}", crate::utils::verbose_solana_error(.error))]
        Solana {
            #[source]
            #[serde_with(as = "Arc<AsClientError>")]
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

    impl From<actix::MailboxError> for Error {
        fn from(value: actix::MailboxError) -> Self {
            CommonError::from(value).into()
        }
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
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct FlowSetContextData {
    pub flow_owner: User,
    pub started_by: User,
    pub endpoints: Endpoints,
    pub solana: SolanaClientConfig,
    pub http: HttpClientConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct FlowContextData {
    pub flow_run_id: FlowRunId,
    pub environment: HashMap<String, String>,
    pub inputs: ValueSet,
    pub set: FlowSetContextData,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct CommandContextData {
    pub node_id: NodeId,
    pub times: u32,
    pub flow: FlowContextData,
}

#[derive(Clone, bon::Builder)]
pub struct FlowSetServices {
    pub http: reqwest::Client,
    pub solana_client: Arc<SolanaClient>,
    pub extensions: Arc<Extensions>,
    pub api_input: api_input::Svc,
}

#[derive(Clone, bon::Builder)]
pub struct FlowServices {
    pub signer: signer::Svc,
    pub set: FlowSetServices,
}

#[derive(Clone, bon::Builder)]
pub struct CommandContext {
    data: CommandContextData,
    execute: execute::Svc,
    get_jwt: get_jwt::Svc,
    node_log: NodeLogSender,
    flow: FlowServices,
}

impl CommandContext {
    pub fn test_context() -> Self {
        let config = ContextConfig::default();
        let solana_client = Arc::new(config.solana_client.build_client(None));
        let node_id = NodeId::nil();
        let times = 0;
        let (tx, _) = flow_run_events::channel();
        Self {
            data: CommandContextData {
                node_id,
                times,
                flow: FlowContextData {
                    flow_run_id: FlowRunId::nil(),
                    environment: HashMap::new(),
                    inputs: ValueSet::default(),
                    set: FlowSetContextData {
                        flow_owner: User::default(),
                        started_by: User::default(),
                        endpoints: Endpoints::default(),
                        solana: config.solana_client,
                        http: config.http_client,
                    },
                },
            },
            execute: unimplemented_svc(),
            get_jwt: unimplemented_svc(),
            node_log: NodeLogSender::new(tx, node_id, times),
            flow: FlowServices {
                signer: unimplemented_svc(),
                set: FlowSetServices {
                    http: reqwest::Client::new(),
                    solana_client,
                    extensions: <_>::default(),
                    api_input: unimplemented_svc(),
                },
            },
        }
    }

    pub fn log(&self, log: NodeLogContent) -> Result<(), mpsc::TrySendError<Event>> {
        self.node_log.send(log)
    }

    pub fn flow_inputs(&self) -> &value::Map {
        &self.data.flow.inputs
    }

    pub fn new_interflow_origin(&self) -> FlowRunOrigin {
        FlowRunOrigin::Interflow {
            flow_run_id: *self.flow_run_id(),
            node_id: *self.node_id(),
            times: *self.times(),
        }
    }

    pub fn flow_run_id(&self) -> &FlowRunId {
        &self.data.flow.flow_run_id
    }

    pub fn node_id(&self) -> &NodeId {
        &self.data.node_id
    }

    pub fn times(&self) -> &u32 {
        &self.data.times
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

    pub fn solana_client(&self) -> &Arc<SolanaClient> {
        &self.flow.set.solana_client
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.flow.set.http
    }

    pub async fn api_input(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<api_input::Response, api_input::Error> {
        let req = api_input::Request {
            flow_run_id: *self.flow_run_id(),
            node_id: *self.node_id(),
            times: *self.times(),
            timeout: timeout.unwrap_or(Duration::MAX),
        };
        self.flow.set.api_input.ready().await?.call(req).await
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

    /// Call [`execute`] service.
    pub async fn execute(
        &mut self,
        instructions: Instructions,
        output: value::Map,
    ) -> Result<execute::Response, execute::Error> {
        self.execute
            .ready()
            .await?
            .call(execute::Request {
                instructions,
                output,
            })
            .await
    }

    /// Call [`signer`] service.
    pub async fn request_signature(
        &mut self,
        pubkey: Pubkey,
        message: Bytes,
        timeout: Duration,
    ) -> Result<signer::SignatureResponse, signer::Error> {
        self.flow
            .signer
            .ready()
            .await?
            .call(signer::SignatureRequest {
                id: None,
                time: Utc::now(),
                pubkey,
                message,
                timeout,
                flow_run_id: Some(self.data.flow.flow_run_id),
                signatures: None,
            })
            .await
    }

    /// Get an extension by type.
    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.flow.set.extensions.get::<T>()
    }

    pub fn extensions_mut(&mut self) -> Option<&mut Extensions> {
        Arc::get_mut(&mut self.flow.set.extensions)
    }

    pub fn raw(&self) -> RawContext<'_> {
        RawContext {
            data: &self.data,
            services: RawServices {
                signer: &self.flow.signer,
                execute: &self.execute,
                get_jwt: &self.get_jwt,
            },
        }
    }
}

pub struct RawServices<'a> {
    pub signer: &'a signer::Svc,
    pub execute: &'a execute::Svc,
    pub get_jwt: &'a get_jwt::Svc,
}

pub struct RawContext<'a> {
    pub data: &'a CommandContextData,
    pub services: RawServices<'a>,
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::test_context()
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, JsonSchema)]
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
