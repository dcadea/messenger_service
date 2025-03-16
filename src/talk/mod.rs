use std::fmt::Display;

use axum::{
    Router,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

use mongodb::bson::serde_helpers::hex_string_as_object_id;

use crate::state::State;

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

impl Id {
    pub fn random() -> Self {
        Self(mongodb::bson::oid::ObjectId::new().to_hex())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn pages<S>(s: State) -> Router<S> {
    Router::new()
        .route("/", get(handler::pages::home))
        .route("/talks/{id}", get(handler::pages::active_talk))
        .with_state(s)
}

pub fn api<S>(s: State) -> Router<S> {
    Router::new()
        .route("/talks/{id}", get(handler::api::find_one))
        .route("/talks", post(handler::api::create))
        .route("/talks/{id}", delete(handler::api::delete))
        .with_state(s)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("talks not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("user is not a member of the talk")]
    NotMember,
    #[error("could not create talk")]
    NotCreated,
    #[error("could not delete talk")]
    NotDeleted,

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}
