use axum::{
    routing::{get, post},
    Router,
};

use crate::{state::AppState, user};

mod handler;
mod markup;
mod model;
pub(crate) mod repository;
pub(crate) mod service;

type Result<T> = std::result::Result<T, Error>;
pub(crate) type Id = mongodb::bson::oid::ObjectId;

pub(crate) fn pages<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/", get(markup::home))
        .route("/chats", get(markup::all_chats))
        .route("/chats/:id", get(markup::active_chat))
        .with_state(state)
}

pub(crate) fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(markup::all))
        .route("/chats/:id", get(markup::one))
        .route("/chats", post(handler::create))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    #[error("chat not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("chat already exists for members: {0:?}")]
    AlreadyExists([user::Sub; 2]),
    #[error("user is not a member of the chat")]
    NotMember,
    #[error("unexpected chat error: {0}")]
    Unexpected(String),

    _User(#[from] user::Error),

    _MongoDB(#[from] mongodb::error::Error),
    _Redis(#[from] redis::RedisError),
}
