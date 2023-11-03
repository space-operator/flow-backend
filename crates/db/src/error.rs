use crate::{connection::proxied_user_conn, StorageError};
use std::panic::Location;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error<E = anyhow::Error> {
    #[error("not supported")]
    NotSupported,
    #[error("failed to create database connection pool:\n{0}")]
    CreatePool(deadpool_postgres::ConfigError),
    #[error("failed to get a database connection from pool:\n{0}")]
    GetDbConnection(deadpool_postgres::PoolError),
    #[error("failed to initialize database tables:\n{0}")]
    InitDb(tokio_postgres::Error),
    #[error("failed to execute statement: {error}, context {context:?}, at {location}")]
    Execute {
        #[source]
        error: tokio_postgres::Error,
        context: &'static str,
        location: &'static Location<'static>,
    },
    #[error("failed to parse data: {error}, context {context:?}, at {location}")]
    Data {
        #[source]
        error: tokio_postgres::Error,
        context: &'static str,
        location: &'static Location<'static>,
    },
    #[error("failed to parse data: {error}, context {context:?}, at {location}")]
    Json {
        #[source]
        error: serde_json::Error,
        context: &'static str,
        location: &'static Location<'static>,
    },
    #[error("{kind} not found: {id}, at {location}")]
    ResourceNotFound {
        kind: &'static str,
        id: String,
        location: &'static Location<'static>,
    },
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("no certificate in PEM file")]
    NoCert,
    #[error("failed to add cert to root-ca: {0}")]
    AddCert(String),
    #[error(transparent)]
    Deserialize(serde_json::Error),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("bcrypt failed")]
    Bcrypt,
    #[error("invalid base58")]
    Base58,
    #[error("{0}")]
    LogicError(E),
    #[error("sled error: {error}, context {context:?}, at {location}")]
    LocalStorage {
        #[source]
        error: kv::Error,
        context: &'static str,
        location: &'static Location<'static>,
    },
    #[error(transparent)]
    ProxyError(#[from] proxied_user_conn::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl<E: Into<anyhow::Error>> Error<E> {
    pub fn erase_type(self) -> Error {
        match self {
            Error::NotSupported => Error::NotSupported,
            Error::LogicError(e) => Error::LogicError(anyhow::anyhow!(e)),
            Error::CreatePool(e) => Error::CreatePool(e),
            Error::GetDbConnection(e) => Error::GetDbConnection(e),
            Error::InitDb(e) => Error::InitDb(e),
            Error::Execute {
                error,
                context,
                location,
            } => Error::Execute {
                error,
                context,
                location,
            },
            Error::Data {
                error,
                context,
                location,
            } => Error::Data {
                error,
                context,
                location,
            },
            Error::Json {
                error,
                context,
                location,
            } => Error::Json {
                error,
                context,
                location,
            },
            Error::ResourceNotFound { kind, id, location } => {
                Error::ResourceNotFound { kind, id, location }
            }
            Error::Io(e) => Error::Io(e),
            Error::NoCert => Error::NoCert,
            Error::AddCert(e) => Error::AddCert(e),
            Error::Deserialize(e) => Error::Deserialize(e),
            Error::Storage(e) => Error::Storage(e),
            Error::Bcrypt => Error::Bcrypt,
            Error::Base58 => Error::Base58,
            Error::LocalStorage {
                error,
                context,
                location,
            } => Error::LocalStorage {
                error,
                context,
                location,
            },
            Error::ProxyError(e) => Error::ProxyError(e),
        }
    }

    /// Local storage (sled) error
    #[track_caller]
    pub fn local(context: &'static str) -> impl FnOnce(kv::Error) -> Self {
        let location = std::panic::Location::caller();

        move |error: kv::Error| Error::LocalStorage {
            context,
            location,
            error,
        }
    }

    /// Error when executing a PG statement.
    #[track_caller]
    pub fn exec(context: &'static str) -> impl FnOnce(tokio_postgres::Error) -> Self {
        let location = std::panic::Location::caller();

        move |error: tokio_postgres::Error| Error::Execute {
            context,
            location,
            error,
        }
    }

    /// Error when parsing data from the database, usually for JSON deserialize error.
    #[track_caller]
    pub fn data(context: &'static str) -> impl FnOnce(tokio_postgres::Error) -> Self {
        let location = std::panic::Location::caller();

        move |error: tokio_postgres::Error| Error::Data {
            context,
            location,
            error,
        }
    }

    /// Error when parsing data from the database, usually for JSON deserialize error.
    #[track_caller]
    pub fn json(context: &'static str) -> impl FnOnce(serde_json::Error) -> Self {
        let location = std::panic::Location::caller();

        move |error: serde_json::Error| Error::Json {
            context,
            location,
            error,
        }
    }

    #[track_caller]
    pub fn not_found<I: std::fmt::Display>(kind: &'static str, id: I) -> Self {
        let location = std::panic::Location::caller();

        Error::ResourceNotFound {
            kind,
            location,
            id: id.to_string(),
        }
    }
}
