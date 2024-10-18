use axum::Extension;
use maud::{html, Markup, Render};

use crate::message::markup::message_input;
use crate::user::markup::{UserHeader, UserSearch};
use crate::user::model::UserInfo;
use messenger_service::markup::Wrappable;

use super::model::ChatDto;
use super::Id;

pub async fn home(logged_user: Extension<UserInfo>) -> Wrappable {
    Wrappable(all_chats(logged_user).await)
}

pub async fn all_chats(logged_user: Extension<UserInfo>) -> Markup {
    html! {
        #chat-window ."flex flex-col h-full" {
            (UserHeader{
                name: &logged_user.name,
                picture: &logged_user.picture,
            })

            (UserSearch{})

            #chat-list
                hx-get="/api/chats"
                hx-trigger="load"
                hx-swap="outerHTML" {}
        }
    }
}

pub fn active_chat(id: &Id, recipient: &UserInfo) -> Markup {
    html! {
        header class="flex justify-between items-center" {
            a class="border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                hx-get="/chats"
                hx-target="#chat-window"
                hx-swap="outerHTML" { "X" }
            h2 class="text-2xl" { (recipient.name) }
            img class="w-12 h-12 rounded-full"
                src=(recipient.picture) alt="User avatar" {}
        }

        div id="active-chat"
            class="flex-grow overflow-y-auto mt-4 mb-4"
        {
            div class="message-list flex flex-col-reverse"
                hx-get={ "/api/messages?limit=14&chat_id=" (id) }
                hx-trigger="load"
                hx-swap="innerHTML" {}
        }

        (message_input(id, &recipient.sub))
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
            div class="chat-item p-4 mb-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex justify-between"
                id={"c-" (self.id)}
                hx-get={"/chats/" (self.id)}
                hx-target="#chat-window" {

                span."chat-recipient font-bold" { (self.recipient) }
                @if let Some(last_message) = &self.last_message {
                    span class="chat-last-message text-sm text-gray-500 truncate" { (last_message) }
                }
            }
        }
    }
}
