use std::fmt::Display;

use axum::{routing::post, Router};
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

mod handler;
pub mod markup;
pub mod middleware;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Deserialize, Serialize)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/users/search", post(handler::search))
        .with_state(state)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Sub(pub String);

impl Sub {
    fn parts(&self) -> (String, String) {
        // split string by '|' and return array of 2 elements
        let mut parts = self.0.splitn(2, '|');
        let provider = parts.next().expect("provider must be present");
        let id = parts.next().expect("id must be present");

        (provider.to_string(), id.to_string())
    }

    pub fn id(&self) -> String {
        self.parts().1
    }
}

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
pub enum Error {
    #[error("user not found: {:?}", 0)]
    NotFound(Sub),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}
