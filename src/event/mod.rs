use axum::routing::get;
use axum::Router;

use crate::state::AppState;
use crate::{auth, chat, message, user};

mod context;
mod handler;
mod markup;
mod model;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

pub fn api<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws", get(handler::ws))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("not a message owner")]
    NotOwner,
    #[error("not a message recipient")]
    NotRecipient,

    _Auth(#[from] auth::Error),
    _Chat(#[from] chat::Error),
    _Message(#[from] message::Error),
    _User(#[from] user::Error),

    _ParseJson(#[from] serde_json::Error),
    _Lapin(#[from] lapin::Error),
    _Redis(#[from] redis::RedisError),
}
