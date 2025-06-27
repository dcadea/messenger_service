use std::{fmt::Display, sync::Arc};

use axum::{Router, routing::post};
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types};
use log::error;
use repository::UserRepository;
use serde::{Deserialize, Serialize};
use service::UserService;
use uuid::Uuid;

use crate::state::AppState;

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn UserRepository + Send + Sync>;
pub type Service = Arc<dyn UserService + Send + Sync>;

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/users/search", post(handler::api::search))
        .with_state(s)
}

#[derive(Clone, Deserialize, Serialize, Hash, PartialEq, Eq, Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Uuid)]
pub struct Id(Uuid);

impl Id {
    pub fn get(&self) -> &Uuid {
        &self.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl From<Uuid> for Id {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<&Uuid> for Id {
    fn from(uuid: &Uuid) -> Self {
        Self(uuid.clone())
    }
}

#[derive(Serialize, Deserialize, FromSqlRow, Clone, Debug, PartialEq, Eq)]
pub struct Sub(String);

impl Sub {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parts(&self) -> (&str, &str) {
        let (provider, id) = {
            let mut parts = self.0.splitn(2, '|');
            let provider = parts.next().expect("provider must be present");
            let id = parts.next().expect("id must be present");
            (provider, id)
        };

        (provider, id)
    }

    pub fn id(&self) -> &str {
        self.parts().1
    }
}

impl From<String> for Sub {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Sub {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Serialize, Deserialize, FromSqlRow, Clone, PartialEq, Eq, Debug)]
pub struct Nickname(String);

impl Nickname {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for Nickname {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Nickname {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Serialize, Deserialize, FromSqlRow, Clone, PartialEq, Eq, Debug)]
pub struct Picture(String);

impl Picture {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for Picture {
    type Error = Error;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        if s.len() == 0 {
            return Err(Self::Error::MalformedPicture(s.to_string()));
        }
        // TODO: parse picture url here
        Ok(Self(s.to_string()))
    }
}

impl Display for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Serialize, Deserialize, FromSqlRow, Clone, PartialEq, Eq, Debug)]
pub struct Email(String);

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for Email {
    type Error = Error;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        if s.len() == 0 {
            return Err(Self::Error::MalformedEmail(s.to_string()));
        }
        // TODO: parse email here
        Ok(Self(s.to_string()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("user not found: {0:?}")]
    NotFound(Sub),
    #[error("invalid picture format: {0:?}")]
    MalformedPicture(String),
    #[error("invalid email format: {0:?}")]
    MalformedEmail(String),
    #[error("authenticated user is not a member")]
    NotMember,

    #[error(transparent)]
    _R2d2(#[from] r2d2::Error),
    #[error(transparent)]
    _Diesel(#[from] diesel::result::Error),
}
