use std::{fmt::Display, sync::Arc};

use axum::{Router, routing::post};
use log::error;
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use repository::UserRepository;
use serde::{Deserialize, Serialize};
use service::UserService;

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

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] String);

impl Id {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
impl Id {
    pub fn random() -> Self {
        Self(mongodb::bson::oid::ObjectId::new().to_hex())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Picture(pub String);

impl Picture {
    pub fn parse(e: &str) -> self::Result<Self> {
        // TODO: parse picture url here
        Ok(Self(e.to_string()))
    }
}

impl Display for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Email(String);

impl Email {
    pub fn parse(e: &str) -> self::Result<Self> {
        // TODO: parse email here
        Ok(Self(e.to_string()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("user not found: {0:?}")]
    NotFound(Sub),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}
