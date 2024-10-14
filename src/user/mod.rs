use std::fmt::Display;

use axum::{routing::post, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

mod handler;
pub mod markup;
pub mod middleware;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
type Id = mongodb::bson::oid::ObjectId;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/users/search", post(handler::search))
        .with_state(state)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Sub(pub String);

impl Display for Sub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    fn deserialize<D>(deserializer: D) -> std::result::Result<Sub, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Sub(s))
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("user not found: {:?}", 0)]
    NotFound(Sub),

    _MongoDB(#[from] mongodb::error::Error),
    _Redis(#[from] redis::RedisError),
    _ParseJson(#[from] serde_json::Error),
}
