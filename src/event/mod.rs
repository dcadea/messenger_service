use std::sync::Arc;

use axum::{
    Router,
    routing::{any, get},
};
use log::error;
use serde::{Deserialize, Serialize};
use service::EventService;

use crate::state::AppServices;
use crate::{message, talk, user};

mod handler;
mod markup;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Service = Arc<dyn EventService + Send + Sync>;

pub fn api<S>(s: AppServices) -> Router<S> {
    Router::new()
        .route("/sse", get(handler::sse::notifications))
        .route("/ws/{talk_id}", any(handler::ws::talk))
        .with_state(s)
}

#[derive(Clone, Debug)]
pub enum Subject<'a> {
    Notifications(&'a user::Id),
    Messages(&'a user::Id, &'a talk::Id),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    OnlineStatusChange(user::model::OnlineStatus),
    NewTalk(talk::model::TalkDto),
    NewMessage {
        talk_id: talk::Id,
        last_message: message::model::MessageDto,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    New(message::model::MessageDto),
    Updated { msg: message::model::MessageDto },
    Deleted { id: message::Id },
    Seen(message::model::MessageDto),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    _NatsSub(#[from] async_nats::SubscribeError),
    #[error(transparent)]
    _SerdeJson(#[from] serde_json::Error),
}
