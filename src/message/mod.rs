use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use log::error;
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use repository::MessageRepository;
use serde::{Deserialize, Serialize};
use service::MessageService;

use crate::state::AppState;

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn MessageRepository + Send + Sync>;
pub type Service = Arc<dyn MessageService + Send + Sync>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

impl Id {
    pub fn random() -> Self {
        Self(mongodb::bson::oid::ObjectId::new().to_hex())
    }
}

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/messages", post(handler::api::create))
        .route("/messages", get(handler::api::find_all))
        .route("/messages", put(handler::api::update))
        .route("/messages/{id}", delete(handler::api::delete))
        .with_state(s)
}

pub fn templates<S>(s: AppState) -> Router<S> {
    Router::new()
        .route(
            "/messages/input/blank",
            get(handler::templates::message_input_blank),
        )
        .route(
            "/messages/input/edit",
            get(handler::templates::message_input_edit),
        )
        .with_state(s)
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

    // FIXME: this is not ok
    #[error("unexpected error occurred: {0:?}")]
    Unexpected(String),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}
