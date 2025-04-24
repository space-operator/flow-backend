use actix::MailboxError;
use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    future::ready,
    sync::Arc,
};
use thiserror::Error as ThisError;
use tower::{service_fn, util::BoxCloneSyncService};

pub type TowerClient<T, U, E> = BoxCloneSyncService<T, U, E>;

#[derive(Clone, Debug, ThisError)]
pub enum CommonError {
    #[error("unimplemented")]
    Unimplemented,
    #[error(transparent)]
    MailBox(#[from] MailboxError),
    #[error(transparent)]
    Other(Arc<anyhow::Error>),
}

pub trait FromAnyhow: Sized {
    fn from_anyhow(e: anyhow::Error) -> Self;
}

pub fn unimplemented_svc<T, U, E>() -> TowerClient<T, U, E>
where
    E: From<CommonError> + Send + 'static,
    U: Send + 'static,
{
    TowerClient::new(service_fn(|_| {
        ready(Err(CommonError::Unimplemented.into()))
    }))
}

pub trait CommonErrorExt {
    fn msg<M: Display + Debug + Send + Sync + 'static>(msg: M) -> Self;
    fn other<E: StdError + Send + Sync + 'static>(error: E) -> Self;
    fn from_anyhow(e: anyhow::Error) -> Self;
    fn from_boxed(error: Box<dyn StdError + Send + Sync + 'static>) -> Self;
}

impl<S> CommonErrorExt for S
where
    S: From<CommonError> + Display + Debug + Send + Sync + 'static,
{
    fn msg<M: Display + Debug + Send + Sync + 'static>(msg: M) -> Self {
        CommonError::Other(Arc::new(anyhow::Error::msg(msg))).into()
    }

    fn other<E: StdError + Send + Sync + 'static>(error: E) -> Self {
        CommonError::Other(Arc::new(anyhow::Error::new(error))).into()
    }

    fn from_boxed(error: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        CommonError::Other(Arc::new(anyhow::Error::from_boxed(error))).into()
    }

    fn from_anyhow(e: anyhow::Error) -> Self {
        e.downcast::<Self>()
            .unwrap_or_else(|error| CommonError::Other(Arc::new(error)).into())
    }
}
