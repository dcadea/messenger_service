use maud::{Markup, Render, html};

use crate::{
    markup::IdExt,
    message::{self, markup::MESSAGE_LIST_ID},
    user,
};

use super::{Message, Notification};

impl Render for Notification {
    fn render(&self) -> Markup {
        match self {
            Self::OnlineStatusChange(os) => user::markup::Icon::OnlineIndicator(os).render(),
            Self::NewTalk(talk_dto) => talk_dto.render(),
            Self::NewMessage {
                talk_id,
                last_message,
            } => message::markup::last_message(Some(last_message), talk_id, None),
        }
    }
}

impl Render for Message {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::New(msg) => div #(MESSAGE_LIST_ID) hx-swap-oob="afterbegin" {
                    (message::markup::MessageItem::new(&msg, None))
                },
                Self::Updated{ msg, auth_id } => (message::markup::MessageItem::new(msg, Some(auth_id))),
                Self::Deleted(id) => div #(id.attr()) ."message-item flex items-center items-baseline" {
                    div ."message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs"
                        ."bg-gray-300 text-gray-600 italic" {
                        "message deleted..."
                    }
                },
                Self::Seen(msg) => div #(msg.id().attr()) hx-swap-oob="beforeend" {
                    (message::markup::Icon::Seen)
                },
            }
        }
    }
}
