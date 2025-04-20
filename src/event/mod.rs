use std::pin::Pin;
use std::sync::Arc;

use axum::{
    Router,
    http::StatusCode,
    response::sse,
    routing::{any, get},
};
use bytes::Bytes;
use futures::Stream;
use log::error;
use maud::{Markup, Render, html};
use serde::{Deserialize, Serialize};
use service::EventService;

use crate::markup::IdExt;
use crate::message::markup::MESSAGE_LIST_ID;
use crate::state::AppState;
use crate::{message, talk, user};

mod handler;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Service = Arc<dyn EventService + Send + Sync>;

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/sse", get(handler::sse::notifications))
        .route("/ws/{talk_id}", any(handler::ws::talk))
        .with_state(s)
}

pub type PayloadStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

#[derive(Clone, Debug)]
pub enum Subject<'a> {
    Notifications(&'a user::Sub),
    Messages(&'a user::Sub, &'a talk::Id),
}

impl async_nats::subject::ToSubject for &Subject<'_> {
    fn to_subject(&self) -> async_nats::Subject {
        match self {
            Subject::Notifications(sub) => format!("noti.{sub}").into(),
            Subject::Messages(sub, talk_id) => format!("messages.{sub}.{talk_id}").into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    OnlineStatusChange(user::model::OnlineStatus),
    NewTalk(talk::model::TalkDto),
    NewMessage {
        talk_id: talk::Id,
        last_message: message::model::LastMessage,
    },
}

impl Render for Notification {
    fn render(&self) -> Markup {
        match self {
            Notification::OnlineStatusChange(os) => {
                user::markup::Icon::OnlineIndicator(os).render()
            }
            Notification::NewTalk(talk_dto) => talk_dto.render(),
            Notification::NewMessage {
                talk_id,
                last_message,
            } => message::markup::last_message(Some(last_message), talk_id, None),
        }
    }
}

impl From<Notification> for sse::Event {
    fn from(noti: Notification) -> Self {
        let evt = match &noti {
            Notification::OnlineStatusChange(f) => &format!("onlineStatusChange:{}", f.id()),
            Notification::NewTalk(_) => "newTalk",
            Notification::NewMessage { talk_id, .. } => &format!("newMessage:{}", &talk_id),
        };

        sse::Event::default()
            .event(evt)
            .data(noti.render().into_string())
    }
}

impl From<Notification> for Bytes {
    fn from(n: Notification) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        if let Err(e) = serde_json::to_writer(&mut bytes, &n) {
            error!("could not serialize notification: {e:?}");
        }
        bytes.into()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    New(message::model::Message),
    Updated {
        msg: message::model::Message,
        auth_sub: user::Sub,
    },
    Deleted(message::Id),
    Seen(message::model::Message),
}

impl Render for Message {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Message::New(msg) => div #(MESSAGE_LIST_ID) hx-swap-oob="afterbegin" {
                    (message::markup::MessageItem::new(&msg, None))
                },
                Message::Updated{ msg, auth_sub } => (message::markup::MessageItem::new(msg, Some(auth_sub))),
                Message::Deleted(id) => div #(id.attr()) ."message-item flex items-center items-baseline" {
                    div ."message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs"
                        ."bg-gray-300 text-gray-600 italic" {
                        "message deleted..."
                    }
                },
                Message::Seen(msg) => div #(msg.id.attr()) hx-swap-oob="beforeend" {
                    (message::markup::Icon::Seen)
                },
            }
        }
    }
}

impl From<Message> for Bytes {
    fn from(m: Message) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        if let Err(e) = serde_json::to_writer(&mut bytes, &m) {
            error!("could not serialize event message: {e:?}");
        }
        bytes.into()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not a message owner")]
    NotOwner,
    #[error("not a message recipient")]
    NotRecipient,

    #[error(transparent)]
    _NatsSub(#[from] async_nats::SubscribeError),
}

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::NotOwner | Error::NotRecipient => StatusCode::FORBIDDEN,
            Error::_NatsSub(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
