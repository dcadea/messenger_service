use axum::{
    routing::{delete, get},
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

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", get(handler::find_all))
        .route("/messages/:id", get(handler::find_one))
        .route("/messages/:id", delete(handler::delete))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("message not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("unexpected message error: {0}")]
    Unexpected(String),

    _MongoDB(#[from] mongodb::error::Error),
}
