use std::pin::Pin;

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::response::sse;
use axum::routing::get;
use futures::Stream;
use maud::{Markup, Render, html};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::user::middleware::cache_user_friends;
use crate::{auth, chat, message, user};

mod handler;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

pub fn api<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/sse", get(handler::sse::notifications))
        .layer(from_fn_with_state(state.clone(), cache_user_friends))
        .route("/ws/{chat_id}", get(handler::ws::chat))
        .with_state(state)
}

pub type PayloadStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

#[derive(Clone, Debug)]
pub enum Subject<'a> {
    Notifications(&'a user::Sub),
    Messages(&'a user::Sub, &'a chat::Id),
}

impl async_nats::subject::ToSubject for &Subject<'_> {
    fn to_subject(&self) -> async_nats::Subject {
        match self {
            Subject::Notifications(sub) => format!("noti.{sub}").into(),
            Subject::Messages(sub, chat_id) => format!("messages.{sub}.{chat_id}").into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    OnlineFriend(user::model::FriendDto),
    NewFriend(chat::model::ChatDto),
    NewMessage {
        chat_id: chat::Id,
        last_message: message::model::LastMessage,
    },
}

impl Render for Notification {
    fn render(&self) -> Markup {
        match self {
            Notification::OnlineFriend(f) => html! { (user::markup::Icon::OnlineStatus(&f)) },
            Notification::NewFriend(chat_dto) => html! { (chat_dto) },
            Notification::NewMessage {
                chat_id,
                last_message,
            } => html! {
                (message::markup::last_message(Some(last_message), chat_id, None))
            },
        }
    }
}

impl From<Notification> for sse::Event {
    fn from(noti: Notification) -> Self {
        let event_name = match &noti {
            Notification::OnlineFriend(f) => &format!("onlineFriend:{}", f.id()),
            Notification::NewFriend(_) => "newFriend",
            Notification::NewMessage { chat_id, .. } => &format!("newMessage:{}", &chat_id),
        };

        sse::Event::default()
            .event(event_name)
            .data(noti.render().into_string())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    New(message::model::Message),
    Updated(message::model::Message),
    Deleted(message::Id),
    Seen(message::model::Message),
}

impl Render for Message {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Message::New(msg) => div #message-list hx-swap-oob="afterbegin" {
                    (message::markup::MessageItem::new(&msg, None))
                },
                Message::Updated(msg) => (message::markup::MessageItem::new(&msg, Some(&msg.recipient))),
                Message::Deleted(id) => div #{"m-" (id)} ."message-item flex items-center items-baseline" {
                    div ."message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs"
                        ."bg-gray-300 text-gray-600 italic" {
                        "message deleted..."
                    }
                },
                Message::Seen(msg) => div #{"m-" (msg._id)} hx-swap-oob="beforeend" {
                    (message::markup::Icon::Seen)
                },
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not a message owner")]
    NotOwner,
    #[error("not a message recipient")]
    NotRecipient,
    #[error("stream unavailable")]
    StreamUnavailable,

    #[error(transparent)]
    _Auth(#[from] auth::Error),
    #[error(transparent)]
    _Chat(#[from] chat::Error),
    #[error(transparent)]
    _User(#[from] user::Error),

    #[error(transparent)]
    _ParseJson(#[from] serde_json::Error),
    #[error(transparent)]
    _NatsPub(#[from] async_nats::PublishError),
    #[error(transparent)]
    _NatsSub(#[from] async_nats::SubscribeError),
    #[error(transparent)]
    _Redis(#[from] redis::RedisError),
}
