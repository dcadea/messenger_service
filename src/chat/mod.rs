use axum::{routing::get, Router};
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Deserialize, Serialize};

use crate::{state::AppState, user};

mod handler;
pub mod markup;
mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

pub fn pages<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/", get(markup::home))
        .route("/chats", get(markup::all_chats))
        .route("/chats/:id", get(handler::open_chat))
        .with_state(state)
}

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(handler::find_all))
        .route("/chats/:id", get(handler::find_one))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("chat not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("chat already exists for members: {0:?}")]
    AlreadyExists([user::Sub; 2]),
    #[error("user is not a member of the chat")]
    NotMember,

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),

    #[error(transparent)]
    _User(#[from] user::Error),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
    #[error(transparent)]
    _Redis(#[from] redis::RedisError),
}
