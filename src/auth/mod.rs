use crate::state::AppState;
use crate::user;
use axum::routing::get;
use axum::Router;
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

pub fn pages<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/login", get(markup::login))
        .with_state(state)
}

pub fn api<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/sso/login", get(handler::login))
        .route("/logout", get(handler::logout))
        .route("/callback", get(handler::callback))
        .with_state(state)
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
    Unexpected(#[from] anyhow::Error),
}
