use std::fmt;
use std::sync::Arc;

use crate::user::model::UserDto;
use crate::user::{self, Email, Picture, Sub};
use crate::{state::AppServices, user::Nickname};
use axum::Router;
use axum::routing::get;
use axum_extra::extract::cookie::Cookie;
use log::error;
use messenger_service::{Raw, Redact};
use serde::Deserialize;

pub mod handler;
pub mod markup;
pub mod middleware;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Service = Arc<dyn service::AuthService + Send + Sync>;

#[derive(Deserialize, Clone)]
struct TokenClaims {
    sub: Sub,
}

pub fn pages<S>(s: AppServices) -> Router<S> {
    Router::new()
        .route("/login", get(handler::pages::login))
        .with_state(s)
}

pub fn api<S>(s: AppServices) -> Router<S> {
    Router::new()
        .route("/sso/login", get(handler::api::sso_login))
        .route("/logout", get(handler::api::logout))
        .route("/callback", get(handler::api::callback))
        .with_state(s)
}

#[derive(Clone)]
pub struct User {
    id: user::Id,
    sub: Sub,
    nickname: Nickname,
    name: String,
    picture: Picture,
}

impl User {
    pub fn new(
        id: user::Id,
        sub: Sub,
        nickname: Nickname,
        name: impl Into<String>,
        picture: Picture,
    ) -> Self {
        Self {
            id,
            sub,
            nickname,
            name: name.into(),
            picture,
        }
    }

    pub const fn id(&self) -> &user::Id {
        &self.id
    }

    pub const fn sub(&self) -> &Sub {
        &self.sub
    }

    pub const fn nickname(&self) -> &Nickname {
        &self.nickname
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn picture(&self) -> &Picture {
        &self.picture
    }
}

impl From<UserDto> for User {
    fn from(u: UserDto) -> Self {
        Self {
            id: u.id().clone(),
            sub: u.sub().clone(),
            nickname: u.nickname().clone(),
            name: u.name().to_string(),
            picture: u.picture().clone(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct UserInfo {
    sub: Sub,
    nickname: Nickname,
    name: String,
    picture: Picture,
    email: Email,
}

impl UserInfo {
    pub const fn sub(&self) -> &Sub {
        &self.sub
    }

    pub const fn nickname(&self) -> &Nickname {
        &self.nickname
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn email(&self) -> &Email {
        &self.email
    }

    pub const fn picture(&self) -> &Picture {
        &self.picture
    }
}

#[derive(Deserialize)]
pub struct Code(String);

impl Code {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Redact for Code {}

impl Raw for Code {
    fn raw(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Code({})", self.redact())
    }
}

#[derive(Deserialize, PartialEq, Eq)]
pub struct Csrf(String);

impl Csrf {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Redact for Csrf {}

impl Raw for Csrf {
    fn raw(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Csrf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Csrf({})", self.redact())
    }
}

#[derive(Deserialize, PartialEq, Eq)]
pub struct Session(String);

impl Session {
    const ID: &str = "session_id";

    pub fn new(sid: impl Into<String>) -> Self {
        Self(sid.into())
    }
}

impl Redact for Session {}

impl Raw for Session {
    fn raw(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Session({})", self.redact())
    }
}

impl From<&Cookie<'_>> for Session {
    fn from(c: &Cookie<'_>) -> Self {
        Self::new(c.value())
    }
}

impl From<Session> for Cookie<'_> {
    fn from(s: Session) -> Self {
        Self::new(Session::ID, s.raw().to_string())
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
    #[error("failed to exchange token")]
    TokenNotExchanged,

    #[error(transparent)]
    _Configuration(#[from] oauth2::ConfigurationError),

    #[error(transparent)]
    _JsonWebtoken(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    _Uuid(#[from] uuid::Error),

    #[error(transparent)]
    _Reqwest(#[from] reqwest::Error),
}
