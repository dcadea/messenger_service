use crate::state::AppState;
use crate::{integration, user};
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
#[error(transparent)]
pub enum Error {
    #[error("unauthorized to access the resource")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("missing or unknown kid")]
    UnknownKid,
    #[error("token is malformed: {0}")]
    TokenMalformed(String),
    #[error("unexpected auth error: {0}")]
    Unexpected(String),
    #[error("invalid state")]
    InvalidState,

    _User(#[from] user::Error),
    _Integration(#[from] integration::Error),

    _Reqwest(#[from] reqwest::Error),
    _ParseJson(#[from] serde_json::Error),
    _Redis(#[from] redis::RedisError),
}
