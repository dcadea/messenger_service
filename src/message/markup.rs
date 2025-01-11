use chrono::DateTime;
use maud::{html, Markup, Render};

use crate::{chat, user};

use super::model::Message;

pub struct MessageInput<'a> {
    chat_id: &'a chat::Id,
    recipient: &'a user::Sub,
}

impl<'a> MessageInput<'a> {
    pub fn new(chat_id: &'a chat::Id, recipient: &'a user::Sub) -> Self {
        Self { chat_id, recipient }
    }
}

impl Render for MessageInput<'_> {
    fn render(&self) -> Markup {
        html! {
            form id="message-input"
                class="border-gray-200 flex"
                hx-post="/api/messages"
                hx-target="#message-list"
                hx-swap="afterbegin"
                _="on htmx:afterRequest reset() me
                   on htmx:afterRequest go to the bottom of the #message-list"
            {
                input type="hidden" name="chat_id" value=(self.chat_id.0) {}
                input type="hidden" name="recipient" value=(self.recipient) {}

                input class="border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none"
                    type="text"
                    name="text"
                    placeholder="Type your message..."
                    _="on keyup if the event's key is 'Escape' set value of me to ''" {}

                input class="bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700"
                    type="submit"
                    value="Send" {}
            }
        }
    }
}

pub struct MessageList<'a> {
    messages: &'a [Message],
    sub: &'a user::Sub,
    append: bool,
}

impl<'a> MessageList<'a> {
    pub fn prepend(messages: &'a [Message], sub: &'a user::Sub) -> Self {
        Self {
            messages,
            sub,
            append: false,
        }
    }

    pub fn append(messages: &'a [Message], sub: &'a user::Sub) -> Self {
        Self {
            messages,
            sub,
            append: true,
        }
    }
}

impl Render for MessageList<'_> {
    fn render(&self) -> Markup {
        let sub = Some(self.sub);
        html! {
            @for i in 0..self.messages.len() {
                @if self.append && i == self.messages.len() - 1 {
                    (MessageItem::new(&self.messages[i], sub).as_last())
                } @else {
                    (MessageItem::new(&self.messages[i], sub))
                }
            }
        }
    }
}

pub struct MessageItem<'a> {
    msg: &'a Message,
    sub: Option<&'a user::Sub>,
    is_last: bool,
}

impl<'a> MessageItem<'a> {
    pub fn new(msg: &'a Message, sub: Option<&'a user::Sub>) -> Self {
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
        let belongs_to_user = match self.sub {
            Some(sub) => self.msg.owner == *sub,
            None => false,
        };

        let message_class = "message-item flex items-end relative";
        let hyperscript = r#"
            on mouseover remove .hidden from my.querySelector('.message-controls')
            on mouseout add .hidden to my.querySelector('.message-controls')
            "#;

        html! {

            @if self.is_last {
                div id={"m-" (self.msg._id.0)}
                    .{(message_class)}
                    .justify-end[belongs_to_user]
                    hx-trigger="intersect once"
                    hx-swap="afterend"
                    hx-get={ "/api/messages?limit=20&chat_id=" (self.msg.chat_id.0) "&end_time=" (self.msg.timestamp) }
                    _=(hyperscript)
                {
                    (message_bubble(self.msg, belongs_to_user))
                }
            } @else {
                div id={"m-" (self.msg._id.0)}
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

fn message_bubble(msg: &Message, belongs_to_user: bool) -> Markup {
    let message_timestamp = DateTime::from_timestamp(msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

    html! {
        @if belongs_to_user {
            div class="message-controls hidden pb-2" {
                i class="fa-trash-can fa-solid text-red-700 cursor-pointer"
                    hx-delete={"/api/messages/" (msg._id.0)}
                    hx-target={"#m-" (msg._id.0)}
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

            p class="message-text break-words overflow-hidden mr-2 whitespace-normal font-light" lang="en" { (msg.text) }
            @if let Some(mt) = message_timestamp {
                span class="message-timestamp text-xs opacity-65" { (mt) }
            }
        }
    }
}
