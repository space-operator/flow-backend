use crate::StorageError;
use actix_web::{ResponseError, http::StatusCode};
use serde::Serialize;
use std::{
    fmt::{Debug, Display},
    panic::Location,
};
use thiserror::Error as ThisError;

#[derive(Serialize, Debug)]
pub struct ErrorBody {
    pub error: String,
}

impl ErrorBody {
    pub fn build<E: ResponseError>(e: &E) -> actix_web::HttpResponse {
        actix_web::HttpResponse::build(e.status_code()).json(ErrorBody {
            error: e.to_string(),
        })
    }
}

impl<E: Debug + Display> ResponseError for Error<E> {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Error::Unauthorized => StatusCode::NOT_FOUND,
            Error::SpawnError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::EncryptionError => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NoEncryptionKey => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NotSupported => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Timeout => StatusCode::INTERNAL_SERVER_ERROR,
            Error::CreatePool(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::GetDbConnection(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::InitDb(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Execute { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Data { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Json { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Parsing { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ResourceNotFound { .. } => StatusCode::NOT_FOUND,
            Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NoCert => StatusCode::INTERNAL_SERVER_ERROR,
            Error::AddCert(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Deserialize(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Bcrypt => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Base58 => StatusCode::INTERNAL_SERVER_ERROR,
            Error::LogicError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::LocalStorage { .. } => todo!(),
            Error::PolarsError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        ErrorBody::build(self)
    }
}

#[derive(Debug, ThisError)]
pub enum Error<E = anyhow::Error> {
    #[error("unauthorized")]
    Unauthorized,
    #[error("spawn error: {}", .0)]
    SpawnError(tokio::task::JoinError),
    #[error("encryption error")]
    EncryptionError,
    #[error("no encryption key")]
    NoEncryptionKey,
    #[error("not supported")]
    NotSupported,
    #[error("time-out")]
    Timeout,
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
    #[error("parsing error: {error}, context {context:?}, at {location}")]
    Parsing {
        #[source]
        error: anyhow::Error,
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
    PolarsError(#[from] polars::error::PolarsError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl<E> From<tokio::task::JoinError> for Error<E> {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::SpawnError(error)
    }
}

impl<E> From<chacha20poly1305::Error> for Error<E> {
    fn from(_: chacha20poly1305::Error) -> Self {
        Self::EncryptionError
    }
}

impl<E: Into<anyhow::Error>> Error<E> {
    pub fn erase_type(self) -> Error {
        match self {
            Error::PolarsError(e) => Error::PolarsError(e),
            Error::Unauthorized => Error::Unauthorized,
            Error::SpawnError(e) => Error::SpawnError(e),
            Error::EncryptionError => Error::EncryptionError,
            Error::NoEncryptionKey => Error::NoEncryptionKey,
            Error::Timeout => Error::Timeout,
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
            Error::Parsing {
                error,
                context,
                location,
            } => Error::Parsing {
                error,
                context,
                location,
            },
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

    /// Error when parsing JSON data from the database.
    #[track_caller]
    pub fn json(context: &'static str) -> impl FnOnce(serde_json::Error) -> Self {
        let location = std::panic::Location::caller();

        move |error: serde_json::Error| Error::Json {
            context,
            location,
            error,
        }
    }

    /// Error when parsing data from the database.
    #[track_caller]
    pub fn parsing<E1: std::error::Error + Send + Sync + 'static>(
        context: &'static str,
    ) -> impl FnOnce(E1) -> Self {
        let location = std::panic::Location::caller();

        move |error: E1| Error::Parsing {
            context,
            location,
            error: error.into(),
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
