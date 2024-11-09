use axum::routing::get;
use axum::Router;

use crate::state::AppState;
use crate::{auth, chat, message, user};

mod handler;
mod markup;
pub mod model;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

pub fn api<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws", get(handler::ws))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not a message owner")]
    NotOwner,
    #[error("not a message recipient")]
    NotRecipient,

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),

    #[error(transparent)]
    _Auth(#[from] auth::Error),
    #[error(transparent)]
    _Chat(#[from] chat::Error),
    #[error(transparent)]
    _Message(#[from] message::Error),
    #[error(transparent)]
    _User(#[from] user::Error),

    #[error(transparent)]
    _ParseJson(#[from] serde_json::Error),
    #[error(transparent)]
    _Lapin(#[from] lapin::Error),
    #[error(transparent)]
    _NatsPub(#[from] async_nats::PublishError),
    #[error(transparent)]
    _NatsSub(#[from] async_nats::SubscribeError),
}
