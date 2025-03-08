use std::fmt::Display;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Deserialize, Serialize};

use crate::{chat, state::AppState};

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

pub fn api<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", post(handler::api::create))
        .route("/messages", get(handler::api::find_all))
        .route("/messages", put(handler::api::update))
        .route("/messages/{id}", delete(handler::api::delete))
        .with_state(state)
}

pub fn templates<S>(state: AppState) -> Router<S> {
    Router::new()
        .route(
            "/messages/input/blank",
            get(handler::templates::message_input_blank),
        )
        .route(
            "/messages/input/edit",
            get(handler::templates::message_input_edit),
        )
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

    #[error(transparent)]
    _Chat(#[from] chat::Error),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}
