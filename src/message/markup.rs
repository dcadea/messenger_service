use chrono::DateTime;
use maud::{html, Markup, Render};

use crate::{chat, user};

use super::model::MessageDto;

pub fn message_input(chat_id: &chat::Id, recipient: &user::Sub) -> Markup {
    html! {
        form id="message-input"
            class="border-gray-200 flex"
            hx-post="/api/messages"
            hx-target="#message-list"
            hx-swap="afterbegin"
            _="on htmx:afterRequest reset() me
               on htmx:afterRequest go to the bottom of the #message-list"
        {
            input type="hidden" name="chat_id" value=(chat_id.0) {}
            input type="hidden" name="recipient" value=(recipient) {}

            input class="border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none"
                type="text"
                name="text"
                placeholder="Type your message..." {}

            input class="bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700"
                type="submit"
                value="Send" {}
        }
    }
}

pub fn message_list(messages: &[MessageDto], logged_sub: &user::Sub) -> Markup {
    html! {
        @for i in 0..messages.len() {
            @if i == messages.len() - 1 {
                (MessageItem::new(&messages[i], logged_sub).as_last())
            } @else {
                (MessageItem::new(&messages[i], logged_sub))
            }
        }
    }
}

pub struct MessageItem<'a> {
    msg: &'a MessageDto,
    sub: &'a user::Sub,
    is_last: bool,
}

impl<'a> MessageItem<'a> {
    pub fn new(msg: &'a MessageDto, sub: &'a user::Sub) -> Self {
        Self {
            msg,
            sub,
            is_last: false,
        }
    }

    pub fn as_last(&'a mut self) -> &'a Self {
        self.is_last = true;
        self
    }
}

impl Render for MessageItem<'_> {
    fn render(&self) -> Markup {
        let belongs_to_user = self.msg.owner == *self.sub;
        let message_class = "message-item flex items-center items-baseline relative";
        let hyperscript = r#"
            on mouseover remove .hidden from my.querySelector('.message-controls')
            on mouseout add .hidden to my.querySelector('.message-controls')
            "#;

        html! {

            @if self.is_last {
                div id={"m-" (self.msg.id.0)}
                    .{(message_class)}
                    .justify-end[belongs_to_user]
                    hx-trigger="intersect once"
                    hx-swap="afterend"
                    hx-get={ "/api/messages?limit=14&chat_id=" (self.msg.chat_id.0) "&end_time=" (self.msg.timestamp) }
                    _=(hyperscript)
                {
                    (message_bubble(self.msg, belongs_to_user))
                }
            } @else {
                div id={"m-" (self.msg.id.0)}
                    .{(message_class)}
                    .justify-end[belongs_to_user]
                    _=(hyperscript)
                {
                    (message_bubble(self.msg, belongs_to_user))
                }
            }
        }
    }
}

pub struct SeenIcon;

impl Render for SeenIcon {
    fn render(&self) -> Markup {
        html! {
            i class="fa-solid fa-check absolute bottom-1 right-2.5 text-white opacity-65" {}
        }
    }
}

struct SentIcon;

impl Render for SentIcon {
    fn render(&self) -> Markup {
        html! {
            i class="fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65" {}
        }
    }
}

fn message_bubble(msg: &MessageDto, belongs_to_user: bool) -> Markup {
    let message_timestamp = DateTime::from_timestamp(msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

    html! {
        @if belongs_to_user {
            div class="message-controls hidden" {
                i class="fa-trash-can fa-solid text-red-700 cursor-pointer"
                    hx-delete={"/api/messages/" (msg.id.0)}
                    hx-target={"#m-" (msg.id.0)}
                    hx-swap="outerHTML" {}

                // TODO: Add edit handler
                i class="fa-pen fa-solid ml-2 text-green-700 cursor-pointer" {}
            }

            (SentIcon)

            @if msg.seen {
                (SeenIcon)
            }
        }

        div ."message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs"
            ."bg-blue-600 text-white ml-2"[belongs_to_user]
            ."bg-gray-300 text-gray-600"[!belongs_to_user] {

            p class="message-text mr-3 whitespace-normal font-light" { (msg.text) }
            @if let Some(mt) = message_timestamp {
                span class="message-timestamp text-xs" { (mt) }
            }
        }
    }
}
