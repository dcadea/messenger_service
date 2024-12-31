use axum::{
    routing::{delete, get, post},
    Router,
};
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

impl Id {
    pub fn random() -> Self {
        Self(mongodb::bson::oid::ObjectId::new().to_hex())
    }
}

pub fn api<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", post(handler::create))
        .route("/messages", get(handler::find_all))
        .route("/messages/:id", delete(handler::delete))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("message not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("not owner of message")]
    NotOwner,
    #[error("message text is empty")]
    EmptyText,

    #[error("message id not present")]
    IdNotPresent,

    _MongoDB(#[from] mongodb::error::Error),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
