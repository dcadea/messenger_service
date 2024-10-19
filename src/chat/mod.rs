use axum::{routing::get, Router};

use crate::{state::AppState, user};

mod handler;
mod markup;
mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Id = mongodb::bson::oid::ObjectId;

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
#[error(transparent)]
pub enum Error {
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
