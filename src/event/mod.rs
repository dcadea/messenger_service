use axum::routing::get;
use axum::Router;

use crate::state::AppState;
use crate::{auth, chat, integration, message, user};

mod context;
mod handler;
mod model;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

pub fn endpoints<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws", get(handler::ws))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("missing user info")]
    MissingUserInfo,
    #[error("not a message owner")]
    NotOwner,
    #[error("not a message recipient")]
    NotRecipient,
    #[error("missing amqp channel")]
    MissingAmqpChannel,

    _Auth(#[from] auth::Error),
    _Chat(#[from] chat::Error),
    _Integration(#[from] integration::Error),
    _Message(#[from] message::Error),
    _User(#[from] user::Error),

    _ParseJson(#[from] serde_json::Error),
    _Lapin(#[from] lapin::Error),
    _Redis(#[from] redis::RedisError),
}
