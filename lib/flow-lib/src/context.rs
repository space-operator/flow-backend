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
    config::{client::FlowRunOrigin, Endpoints},
    solana::Instructions,
    utils::Extensions,
    ContextConfig, FlowRunId, NodeId, UserId,
};
use bytes::Bytes;
use solana_client::nonblocking::rpc_client::RpcClient as SolanaClient;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::{any::Any, collections::HashMap, sync::Arc, time::Duration};
use tower::{Service, ServiceExt};

/// Get user's JWT, require
/// [`user_token`][crate::config::node::Permissions::user_tokens] permission.
pub mod get_jwt {
    use crate::{utils::TowerClient, BoxError, UserId};
    use std::sync::Arc;
    use thiserror::Error as ThisError;

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
        #[error(transparent)]
        Worker(Arc<BoxError>),
        #[error(transparent)]
        MailBox(#[from] Arc<actix::MailboxError>),
        #[error(transparent)]
        Other(#[from] Arc<BoxError>),
    }

    impl From<actix::MailboxError> for Error {
        fn from(error: actix::MailboxError) -> Self {
            Error::MailBox(Arc::new(error))
        }
    }

    impl Error {
        pub fn worker(e: BoxError) -> Self {
            Error::Other(Arc::new(e))
        }

        pub fn other<E: Into<BoxError>>(e: E) -> Self {
            Error::Other(Arc::new(e.into()))
        }
    }

    impl actix::Message for Request {
        type Result = Result<Response, Error>;
    }

    pub type Svc = TowerClient<Request, Response, Error>;

    pub fn unimplemented_svc() -> Svc {
        Svc::unimplemented(|| Error::other("unimplemented"), Error::worker)
    }

    pub fn not_allowed() -> Svc {
        Svc::unimplemented(|| Error::NotAllowed, Error::worker)
    }
}

/// Request Solana signature from external wallets.
pub mod signer {
    use crate::{utils::TowerClient, BoxError, UserId};
    use solana_sdk::{pubkey::Pubkey, signature::Signature};
    use std::time::Duration;
    use thiserror::Error as ThisError;

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error("can't sign for this pubkey")]
        Pubkey,
        #[error("can't sign for this user")]
        User,
        #[error("timeout")]
        Timeout,
        #[error(transparent)]
        Worker(BoxError),
        #[error(transparent)]
        MailBox(#[from] actix::MailboxError),
        #[error(transparent)]
        Other(#[from] BoxError),
    }

    pub type Svc = TowerClient<SignatureRequest, SignatureResponse, Error>;

    #[derive(Debug, Clone)]
    pub struct SignatureRequest {
        pub user_id: UserId,
        pub pubkey: Pubkey,
        pub message: bytes::Bytes,
        pub timeout: Duration,
    }

    impl actix::Message for SignatureRequest {
        type Result = Result<SignatureResponse, Error>;
    }

    #[derive(Debug)]
    pub struct SignatureResponse {
        pub signature: Signature,
    }

    pub fn unimplemented_svc() -> Svc {
        Svc::unimplemented(|| BoxError::from("unimplemented").into(), Error::Worker)
    }
}

/// Output values and Solana instructions to be executed.
pub mod execute {
    use crate::{solana::Instructions, utils::TowerClient, BoxError};
    use futures::channel::oneshot::Canceled;
    use solana_client::client_error::ClientError;
    use solana_sdk::{signature::Signature, signer::SignerError};
    use std::sync::Arc;
    use thiserror::Error as ThisError;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub instructions: Instructions,
        pub output: value::Map,
    }

    #[derive(Clone, Copy)]
    pub struct Response {
        pub signature: Option<Signature>,
    }

    #[derive(ThisError, Debug, Clone)]
    pub enum Error {
        #[error("canceled")]
        Canceled,
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
        #[error("{}", crate::solana::verbose_solana_error(.0))]
        Solana(#[from] Arc<ClientError>),
        #[error(transparent)]
        Signer(#[from] Arc<SignerError>),
        #[error(transparent)]
        Worker(Arc<BoxError>),
        #[error(transparent)]
        MailBox(#[from] actix::MailboxError),
        #[error(transparent)]
        ChannelClosed(#[from] Canceled),
        #[error(transparent)]
        Other(#[from] Arc<BoxError>),
    }

    impl From<anyhow::Error> for Error {
        fn from(value: anyhow::Error) -> Self {
            value.downcast::<Self>().unwrap_or_else(Self::other)
        }
    }

    impl From<ClientError> for Error {
        fn from(value: ClientError) -> Self {
            Error::Solana(Arc::new(value))
        }
    }

    impl From<BoxError> for Error {
        fn from(value: BoxError) -> Self {
            Error::Other(Arc::new(value))
        }
    }

    impl From<SignerError> for Error {
        fn from(value: SignerError) -> Self {
            Error::Signer(Arc::new(value))
        }
    }

    impl Error {
        pub fn worker(e: BoxError) -> Self {
            Error::Worker(Arc::new(e))
        }

        pub fn other<E: Into<BoxError>>(e: E) -> Self {
            Error::Other(Arc::new(e.into()))
        }
    }

    pub fn unimplemented_svc() -> Svc {
        Svc::unimplemented(|| Error::other("unimplemented"), Error::worker)
    }

    pub fn simple(ctx: &super::Context, size: usize) -> Svc {
        let rpc = ctx.solana_client.clone();
        let signer = ctx.signer.clone();
        let user_id = ctx.user.id;
        let handle = move |req: Request| {
            let rpc = rpc.clone();
            let signer = signer.clone();
            async move {
                Ok(Response {
                    signature: Some(req.instructions.execute(&rpc, signer, user_id).await?),
                })
            }
        };
        Svc::from_service(tower::service_fn(handle), Error::worker, size)
    }
}

#[derive(Clone)]
pub struct CommandContext {
    pub svc: execute::Svc,
    pub flow_run_id: FlowRunId,
    pub node_id: NodeId,
    pub times: u32,
}

#[derive(Clone)]
pub struct Context {
    pub cfg: ContextConfig,
    pub http: reqwest::Client,
    pub solana_client: Arc<SolanaClient>,
    pub environment: HashMap<String, String>,
    pub user: User,
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
            signer::unimplemented_svc(),
            get_jwt::unimplemented_svc(),
            Extensions::default(),
        );
        ctx.command = Some(CommandContext {
            svc: execute::simple(&ctx, 1),
            flow_run_id: uuid::Uuid::nil(),
            node_id: uuid::Uuid::nil(),
            times: 0,
        });
        ctx
    }
}

#[derive(Clone)]
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
        user: User,
        sig_svc: signer::Svc,
        token_svc: get_jwt::Svc,
        extensions: Extensions,
    ) -> Self {
        let solana_client = SolanaClient::new(cfg.solana_client.url.clone());

        Self {
            cfg: cfg.clone(),
            http: reqwest::Client::new(),
            solana_client: Arc::new(solana_client),
            environment: cfg.environment.clone(),
            user,
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
                    user_id: self.user.id,
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
    ) -> Result<Signature, anyhow::Error> {
        let mut s = self.signer.clone();
        let user_id = self.user.id;

        let signer::SignatureResponse { signature } = s
            .ready()
            .await?
            .call(signer::SignatureRequest {
                user_id,
                pubkey,
                message,
                timeout,
            })
            .await?;
        Ok(signature)
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
