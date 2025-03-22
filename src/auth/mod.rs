use crate::state::State;
use crate::user;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use log::error;
use serde::Deserialize;

mod handler;
mod markup;
pub mod middleware;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

const SESSION_ID: &str = "session_id";

#[derive(Deserialize, Clone)]
struct TokenClaims {
    sub: user::Sub,
}

pub fn pages<S>(s: State) -> Router<S> {
    Router::new()
        .route("/login", get(handler::pages::login))
        .with_state(s)
}

pub fn api<S>(s: State) -> Router<S> {
    Router::new()
        .route("/sso/login", get(handler::api::sso_login))
        .route("/logout", get(handler::api::logout))
        .route("/callback", get(handler::api::callback))
        .with_state(s)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unauthorized to access the resource")]
    Unauthorized,
    #[error("forbidden to access the resource")]
    Forbidden,
    #[error("missing or unknown kid")]
    UnknownKid,
    #[error("token is malformed")]
    TokenMalformed,
    #[error("invalid state")]
    InvalidState,

    #[error(transparent)]
    _Configuration(#[from] oauth2::ConfigurationError),

    #[error(transparent)]
    _JsonWebtoken(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    _Uuid(#[from] uuid::Error),

    #[error(transparent)]
    _Reqwest(#[from] reqwest::Error),

    #[error("unexpected error happened: {0}")]
    _Unexpected(String),
}

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Forbidden | Error::UnknownKid | Error::InvalidState => StatusCode::FORBIDDEN,
            Error::TokenMalformed => StatusCode::BAD_REQUEST,
            Error::_Configuration(_)
            | Error::_JsonWebtoken(_)
            | Error::_Uuid(_)
            | Error::_Reqwest(_)
            | Error::_Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
