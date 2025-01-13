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
                class="flex flex-col space-y-2 h-full overflow-y-auto"
                hx-get="/api/chats"
                hx-trigger="load" {}
        }
    }
}

struct Header<'a>(&'a UserInfo);

impl Render for Header<'_> {
    fn render(&self) -> Markup {
        html! {
            header id="recipient-header"
                class="flex justify-between items-center relative" {
                a class="cursor-pointer border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                    hx-get="/chats"
                    hx-target="#chat-window"
                    hx-swap="outerHTML" { "X" }
                h2 class="text-2xl" { (self.0.name) }
                span class="online-status absolute inset-12 flex items-center justify-center text-xs text-gray-500" { "offline" }
                img class="w-12 h-12 rounded-full"
                    src=(self.0.picture) alt="User avatar" {}
            }
        }
    }
}

pub fn active_chat(id: &Id, recipient: &UserInfo) -> Markup {
    html! {
        (Header(recipient))

        div id="active-chat"
            class="flex-grow overflow-auto mt-4 mb-4"
            ws-connect={ "/ws/" (id.0) }
        {
            div id="message-list"
                class="sticky flex flex-col-reverse overflow-auto h-full"
                hx-get={ "/api/messages?limit=20&chat_id=" (id.0) }
                hx-trigger="load" {}
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
            div class="chat-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                id={"c-" (self.id.0)}
                hx-get={"/chats/" (self.id.0)}
                hx-target="#chat-window"
            {
                // TODO: wrap in green circle when online
                img class="w-8 h-8 rounded-full"
                    src=(self.recipient_picture) alt="Recipient avatar" {}

                span class="chat-recipient font-bold mx-2" { (self.recipient_name) }

                @if let Some(last_message) = &self.last_message {
                    (last_message)

                    @if !last_message.seen && last_message.recipient == self.sender {
                        (UnseenIcon)
                    }
                }
            }
        }
    }
}

pub struct UnseenIcon;

impl Render for UnseenIcon {
    fn render(&self) -> Markup {
        html! {
            i class="fa-solid fa-envelope text-green-600 ml-2" {}
        }
    }
}
