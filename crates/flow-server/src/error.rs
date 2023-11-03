use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error as ThisError;

use crate::db_worker::user_worker;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Flow(#[from] flow::Error),
    #[error(transparent)]
    Db(#[from] db::Error),
    #[error(transparent)]
    SignatureAuth(#[from] crate::user::Invalid),
    #[error(transparent)]
    Login(#[from] crate::user::LoginError),
    #[error("not found")]
    NotFound,
    #[error("{}", msg)]
    Custom { status: StatusCode, msg: String },
    #[error(transparent)]
    Actix(#[from] actix_web::Error),
    #[error(transparent)]
    Mailbox(#[from] actix::MailboxError),
    #[error(transparent)]
    Start(#[from] user_worker::StartError),
    #[error(transparent)]
    CloneFlow(#[from] user_worker::CloneFlowError),
}

impl Error {
    pub fn custom<T: std::fmt::Display>(status: StatusCode, msg: T) -> Self {
        Error::Custom {
            status,
            msg: msg.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Debug)]
pub struct ErrorBody {
    pub error: String,
}

impl ErrorBody {
    pub fn build<E: ResponseError>(e: &E) -> HttpResponse {
        HttpResponse::build(e.status_code()).json(ErrorBody {
            error: e.to_string(),
        })
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Start(e) => e.status_code(),
            Error::SignatureAuth(_) | Error::Login(_) => StatusCode::UNAUTHORIZED,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Custom { status, .. } => *status,
            Error::Actix(e) => e.as_response_error().status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Error::Start(e) => e.error_response(),
            _ => ErrorBody::build(self),
        }
    }
}

#[derive(ThisError, Debug)]
#[error("missing apikey")]
pub struct ApiKey;

impl ResponseError for ApiKey {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse {
        ErrorBody::build(self)
    }
}
