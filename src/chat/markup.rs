use axum::Extension;
use maud::{html, Markup, Render};

use crate::message::markup::MessageInput;
use crate::user::model::UserInfo;
use crate::{message, user};
use messenger_service::markup::Wrappable;

use super::model::ChatDto;
use super::Id;

pub async fn home(logged_user: Extension<UserInfo>) -> Wrappable {
    Wrappable::new(all_chats(logged_user).await).with_sse()
}

pub async fn all_chats(logged_user: Extension<UserInfo>) -> Markup {
    html! {
        div id="chat-window"
            class="flex flex-col h-full"
        {
            (user::markup::Header(&logged_user))

            (user::markup::Search)

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
                    href="/" { "X" }
                h2 class="text-2xl" { (self.0.name) }
                span class="online-status absolute inset-12 flex items-center justify-center text-xs text-gray-500" { "offline" }
                (icon::ChatControls)
            }
        }
    }
}

pub fn active_chat(id: &Id, recipient: &UserInfo) -> Markup {
    html! {
        (Header(recipient))

        div id="active-chat"
            class="flex-grow overflow-auto mt-4 mb-4"
            hx-ext="ws" ws-connect={ "/ws/" (id) }
        {
            div id="message-list"
                class="sticky flex flex-col-reverse overflow-auto h-full"
                hx-get={ "/api/messages?limit=20&chat_id=" (id) }
                hx-trigger="load" {}
        }

        (MessageInput::new(id, &recipient.sub))

        (ChatControls(id))
    }
}

struct ChatControls<'a>(&'a Id);

impl Render for ChatControls<'_> {
    fn render(&self) -> Markup {
        let controls_item_class = "text-lg py-3 cursor-pointer hover:bg-gray-300";

        html! {
            div id="chat-controls"
                ."flex flex-row h-full w-full absolute top-0 left-0 invisible" {
                div class="chat-controls-overlay w-2/3 bg-gray-300 bg-opacity-50"
                    _="on click add .invisible to #chat-controls" {}

                div class="flex flex-col bg-white h-full w-1/3 py-4 text-center" {
                    div class="text-2xl py-3" { "Settings" }
                    div class=(controls_item_class)
                        hx-delete={"/api/chats/" (self.0)} { "Delete chat" }
                }
            }
        }
    }
}

pub fn chat_list(chats: &[ChatDto]) -> Markup {
    html! {
        @for chat in chats {
            (chat)
        }

        i id="noti-bell"
            ."fa-regular fa-bell-slash"
            ."text-green-700 text-3xl"
            ."absolute right-5 bottom-5"
            _="on click askNotificationPermission()"{}
    }
}

impl Render for ChatDto {
    fn render(&self) -> Markup {
        html! {
            div class="chat-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                id={"c-" (self.id)}
                hx-get={"/chats/" (self.id)}
                hx-target="#chat-window"
            {
                // TODO: wrap in green circle when online
                img class="w-8 h-8 rounded-full"
                    src=(self.recipient_picture) alt="Recipient avatar" {}

                span class="chat-recipient font-bold mx-2" { (self.recipient_name) }

                (message::markup::last_message(self.last_message.as_ref(), &self.id, Some(&self.sender)))
            }
        }
    }
}

pub mod icon {
    use maud::{html, Markup, Render};

    pub struct ChatControls;

    impl Render for ChatControls {
        fn render(&self) -> Markup {
            html! {
                i class="fa-solid fa-bars text-2xl cursor-pointer"
                    _="on click toggle .invisible on #chat-controls" {}
            }
        }
    }

    pub struct Unseen;

    impl Render for Unseen {
        fn render(&self) -> Markup {
            html! {
                i class="fa-solid fa-envelope text-green-600 ml-2" {}
            }
        }
    }
}
