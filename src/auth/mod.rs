use std::sync::Arc;

use crate::state::AppState;
use crate::user;
use crate::user::model::UserInfo;
use axum::Router;
use axum::routing::get;
use log::error;
use serde::Deserialize;

pub mod handler;
pub mod markup;
pub mod middleware;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Service = Arc<dyn service::AuthService + Send + Sync>;

const SESSION_ID: &str = "session_id";

#[derive(Deserialize, Clone)]
struct TokenClaims {
    sub: user::Sub,
}

pub fn pages<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/login", get(handler::pages::login))
        .with_state(s)
}

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/sso/login", get(handler::api::sso_login))
        .route("/logout", get(handler::api::logout))
        .route("/callback", get(handler::api::callback))
        .with_state(s)
}

#[derive(Clone)]
pub struct User {
    sub: user::Sub,
    nickname: String,
    name: String,
    picture: String,
}

impl User {
    pub fn new(
        sub: user::Sub,
        nickname: impl Into<String>,
        name: impl Into<String>,
        picture: impl Into<String>,
    ) -> Self {
        User {
            sub,
            nickname: nickname.into(),
            name: name.into(),
            picture: picture.into(),
        }
    }

    pub fn sub(&self) -> &user::Sub {
        &self.sub
    }

    pub fn nickname(&self) -> &str {
        &self.nickname
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn picture(&self) -> &str {
        &self.picture
    }
}

impl From<UserInfo> for User {
    fn from(user_info: UserInfo) -> Self {
        User::new(
            user_info.sub().clone(),
            user_info.nickname(),
            user_info.name(),
            user_info.picture(),
        )
    }
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
    Unexpected(String),
}
