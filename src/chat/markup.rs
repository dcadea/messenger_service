use axum::Extension;
use maud::{html, Markup, Render};

use crate::message::markup::MessageInput;
use crate::user;
use crate::user::model::UserInfo;
use messenger_service::markup::Wrappable;

use super::model::ChatDto;
use super::Id;

pub async fn home(logged_user: Extension<UserInfo>) -> Wrappable {
    Wrappable::new(all_chats(logged_user).await).with_ws()
}

pub async fn all_chats(logged_user: Extension<UserInfo>) -> Markup {
    html! {
        div id="chat-window"
            class="flex flex-col h-full"
        {
            (user::markup::Header(&logged_user))

            (user::markup::Search{})

            div id="chat-list"
                class="flex flex-col space-y-2"
                hx-get="/api/chats"
                hx-trigger="load" {}
        }
    }
}

pub fn active_chat(id: &Id, recipient: &UserInfo) -> Markup {
    html! {
        header id="recipient-header"
            class="flex justify-between items-center relative" {
            a class="border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                hx-get="/chats"
                hx-target="#chat-window"
                hx-swap="outerHTML" { "X" }
            h2 class="text-2xl" { (recipient.name) }
            span class="online-status absolute inset-12 flex items-center justify-center text-xs text-gray-500" { "offline" }
            img class="w-12 h-12 rounded-full"
                src=(recipient.picture) alt="User avatar" {}
        }

        div id="active-chat"
            class="flex-grow overflow-y-auto mt-4 mb-4"
        {
            div id="message-list"
                class="flex flex-col-reverse"
                hx-get={ "/api/messages?limit=14&chat_id=" (id.0) }
                hx-trigger="load" // FIXME: always scrolls to the bottom on next page
                // _="on htmx:afterOnLoad go to the bottom of the #message-list" {}
                // FIXME: custom event doesn't behave as expected
                _="on msg:firstBatch go to the bottom of the #message-list" {}
        }

        (MessageInput::new(id, &recipient.sub))
    }
}

pub fn chat_list(chats: &[ChatDto]) -> Markup {
    html! {
        @for chat in chats {
            (chat)
        }
    }
}

impl Render for ChatDto {
    fn render(&self) -> Markup {
        html! {
            div class="chat-item p-4 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                id={"c-" (self.id.0)}
                hx-get={"/chats/" (self.id.0)}
                hx-target="#chat-window"
            {
                (OfflineIcon { sub: &self.recipient, swappable: false })
                span class="chat-recipient font-bold" { (self.recipient_name) }
                @if let Some(last_message) = &self.last_message {
                    span class="chat-last-message flex-grow text-sm text-gray-500 text-right truncate" {
                        (last_message)
                    }
                }
            }
        }
    }
}

struct OnlineStatusIcon<'a> {
    sub: &'a user::Sub,
    icon: &'a str,
    swappable: bool,
}
pub struct OnlineIcon<'a> {
    pub sub: &'a user::Sub,
    pub swappable: bool,
}
pub struct OfflineIcon<'a> {
    pub sub: &'a user::Sub,
    pub swappable: bool,
}

impl Render for OnlineStatusIcon<'_> {
    fn render(&self) -> Markup {
        let i_class = format!("online-status fa-circle text-green-600 mr-2 {}", self.icon);

        html! {
            @if self.swappable {
                i id={"os-" (self.sub.id()) } hx-swap-oob="true" class=(i_class)  {}
            } @else {
                i id={"os-" (self.sub.id()) } class=(i_class) {}
            }
        }
    }
}

impl Render for OnlineIcon<'_> {
    fn render(&self) -> Markup {
        html! {(OnlineStatusIcon { sub: self.sub, icon: "fa-solid", swappable: self.swappable })}
    }
}

impl Render for OfflineIcon<'_> {
    fn render(&self) -> Markup {
        html! {(OnlineStatusIcon { sub: self.sub, icon: "fa-regular", swappable: self.swappable })}
    }
}
