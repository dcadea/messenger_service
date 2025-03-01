use std::rc::Rc;

use maud::{Markup, Render, html};
use messenger_service::markup::Id;

use crate::message::markup::{MESSAGE_INPUT_TARGET, MESSAGE_LIST_ID};
use crate::user::model::UserInfo;
use crate::{chat, message, user};

use super::model::ChatDto;

const CHAT_WINDOW_ID: &str = "chat-window";
pub const CHAT_WINDOW_TARGET: &str = "#chat-window";

pub struct ChatWindow<'a> {
    user_info: &'a UserInfo,
    chats: Rc<[ChatDto]>,
}

impl<'a> ChatWindow<'a> {
    pub fn new(user_info: &'a UserInfo, chats: &[ChatDto]) -> Self {
        Self {
            user_info,
            chats: chats.into(),
        }
    }

    fn get_chats(&self) -> &[ChatDto] {
        &self.chats
    }
}

impl Render for ChatWindow<'_> {
    fn render(&self) -> Markup {
        html! {
            div #(CHAT_WINDOW_ID) ."flex flex-col h-full" {
                (user::markup::Header(self.user_info))
                (user::markup::Search)
                (ChatList::new(self.get_chats()))
            }
        }
    }
}

struct ChatList(Rc<[ChatDto]>);

impl ChatList {
    fn new(chats: &[ChatDto]) -> Self {
        Self(chats.into())
    }

    fn get_chats(&self) -> &[ChatDto] {
        &self.0
    }
}

impl Render for ChatList {
    fn render(&self) -> Markup {
        html! {
            div #chat-list ."flex flex-col space-y-2 h-full overflow-y-auto"
                sse-swap="newFriend"
                hx-swap="beforeend"
            {
                @for chat in self.get_chats() {
                    (chat)
                }

                i #noti-bell
                    ."fa-regular fa-bell-slash"
                    ."text-green-700 text-3xl"
                    ."absolute right-5 bottom-5"
                    _="on click askNotificationPermission()"{}
            }
        }
    }
}

struct Header<'a>(&'a UserInfo);

impl Render for Header<'_> {
    fn render(&self) -> Markup {
        html! {
            header #recipient-header ."flex justify-between items-center" {
                a ."cursor-pointer border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                    href="/" { "X" }
                ."flex text-2xl" {
                    (user::model::OnlineStatus::new(self.0.sub.clone(), false))
                    (self.0.name)
                }
                (Icon::ChatControls)
            }
        }
    }
}

pub struct ActiveChat<'a> {
    pub id: &'a chat::Id,
    pub recipient: &'a UserInfo,
}

impl Render for ActiveChat<'_> {
    fn render(&self) -> Markup {
        html! {
            (Header(self.recipient))

            div #active-chat ."flex-grow overflow-auto mt-4 mb-4"
                hx-ext="ws"
                ws-connect={ "/ws/" (self.id) }
            {
                div #(MESSAGE_LIST_ID) ."sticky flex flex-col-reverse overflow-auto h-full"
                    hx-get={ "/api/messages?limit=20&chat_id=" (self.id) }
                    hx-trigger="load" {}
            }

            (message::markup::InputBlank::new(self.id, &self.recipient.sub))
            (ChatControls(self.id))

            div .hidden
                hx-trigger="msg:afterUpdate from:body"
                hx-target=(MESSAGE_INPUT_TARGET)
                hx-swap="outerHTML"
                hx-get={"/templates/messages/input/blank"
                    "?chat_id=" (self.id)
                    "&recipient=" (&self.recipient.sub)
                } {}
        }
    }
}

struct ChatControls<'a>(&'a chat::Id);

impl Render for ChatControls<'_> {
    fn render(&self) -> Markup {
        let controls_item_class = "text-lg py-3 cursor-pointer hover:bg-gray-300";

        html! {
            div #chat-controls ."flex flex-row h-full w-full absolute top-0 left-0 invisible" {
                div ."chat-controls-overlay w-2/3 bg-gray-300 bg-opacity-50"
                    _="on click add .invisible to #chat-controls" {}

                div ."flex flex-col bg-white h-full w-1/3 py-4 text-center" {
                    div ."text-2xl py-3" { "Settings" }
                    div .(controls_item_class)
                        hx-delete={"/api/chats/" (self.0)} { "Delete chat" }
                }
            }
        }
    }
}

impl Render for ChatDto {
    fn render(&self) -> Markup {
        html! {
            div #(self.id.attr())
                ."chat-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                hx-get={"/chats/" (self.id)}
                hx-target=(CHAT_WINDOW_TARGET)
                hx-swap="innerHTML"
            {
                (user::model::OnlineStatus::new(self.recipient.clone(), false))
                img ."w-8 h-8 rounded-full"
                    src=(self.recipient_picture) alt="Recipient avatar" {}

                span ."chat-recipient font-bold mx-2" { (self.recipient_name) }

                div ."flex-grow text-right truncate"
                    sse-swap={"newMessage:"(self.id)}
                    hx-target={"#lm-"(self.id)}
                {
                    (message::markup::last_message(self.last_message.as_ref(), &self.id, Some(&self.sender)))
                }
            }
        }
    }
}

impl messenger_service::markup::Id for chat::Id {
    fn attr(&self) -> String {
        format!("c-{}", self.0)
    }

    fn target(&self) -> String {
        format!("#c-{}", self.0)
    }
}

pub enum Icon {
    ChatControls,
    Unseen,
}

impl Render for Icon {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::ChatControls => {
                    i ."fa-solid fa-bars text-2xl cursor-pointer"
                        _="on click toggle .invisible on #chat-controls" {}
                },
                Self::Unseen => i ."fa-solid fa-envelope text-green-600 ml-2" {}
            }

        }
    }
}
