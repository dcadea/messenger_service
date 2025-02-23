use std::rc::Rc;

use maud::{Markup, Render, html};

use crate::user::model::UserInfo;
use crate::{message, user};

use super::Id;
use super::model::ChatDto;

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
            div #chat-window ."flex flex-col h-full" {
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
            header #recipient-header ."flex justify-between items-center relative" {
                a ."cursor-pointer border-2 border-red-500 text-red-500 px-4 py-2 rounded-2xl mr-4"
                    href="/" { "X" }
                h2 .text-2xl { (self.0.name) }
                // TODO: online users feature
                span ."online-status absolute inset-12 flex items-center justify-center text-xs text-gray-500" { "offline" }
                (Icon::ChatControls)
            }
        }
    }
}

pub struct ActiveChat<'a> {
    pub id: &'a Id,
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
                div #message-list ."sticky flex flex-col-reverse overflow-auto h-full"
                    hx-get={ "/api/messages?limit=20&chat_id=" (self.id) }
                    hx-trigger="load" {}
            }

            (message::markup::InputBlank::new(self.id, &self.recipient.sub))
            (ChatControls(self.id))

            div .hidden
                hx-trigger="msg:afterUpdate from:body"
                hx-target="#message-input"
                hx-swap="outerHTML"
                hx-get={"/templates/messages/input/blank"
                    "?chat_id=" (self.id)
                    "&recipient=" (&self.recipient.sub)
                } {}
        }
    }
}

struct ChatControls<'a>(&'a Id);

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
            div #{"c-" (self.id)}
                ."chat-item px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center"
                hx-get={"/chats/" (self.id)}
                hx-target="#chat-window"
                hx-swap="innerHTML"
            {
                (user::model::FriendDto::new(self.recipient.clone(), false))
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
