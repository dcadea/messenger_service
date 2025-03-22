use std::{fmt::Display, sync::Arc};

use axum::{Router, http::StatusCode, routing::post};
use log::error;
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Deserialize, Serialize};

use crate::state::State;

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Deserialize, Serialize)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

pub fn api<S>(s: State) -> Router<S> {
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
        let mut parts = self.0.splitn(2, '|');
        let provider = parts.next().expect("provider must be present");
        let id = parts.next().expect("id must be present");

        (provider, id)
    }

    pub fn id(&self) -> &str {
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
        Ok(Sub(s.into()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("user not found: {0:?}")]
    NotFound(Sub),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::_MongoDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
