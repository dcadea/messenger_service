use std::sync::Arc;

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

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

#[cfg(test)]
impl Id {
    pub fn random() -> Self {
        Self(mongodb::bson::oid::ObjectId::new().to_hex())
    }
}

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/users/search", post(handler::api::search))
        .with_state(s)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Sub(pub Arc<str>);

impl Sub {
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

impl Serialize for Sub {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Sub {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s.into()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("user not found: {0:?}")]
    NotFound(Sub),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}
