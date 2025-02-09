use chrono::DateTime;
use maud::{html, Markup, Render};

use crate::{chat, user};

use super::{
    model::{LastMessage, Message},
    Id,
};

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
            form #message-input ."border-gray-200 flex"
                hx-post="/api/messages"
                hx-target="#message-list"
                hx-swap="afterbegin"
                _="on htmx:afterRequest reset() me
                   on htmx:afterRequest go to the bottom of the #message-list"
            {
                input type="hidden" name="chat_id" value=(self.chat_id) {}
                input type="hidden" name="recipient" value=(self.recipient) {}

                input ."border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none"
                    type="text"
                    name="text"
                    placeholder="Type your message..."
                    autocomplete="off"
                    _="on keyup if the event's key is 'Escape' set value of me to ''" {}

                input ."bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700"
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

    fn as_last(&'a mut self) -> &'a Self {
        self.is_last = true;
        self
    }

    fn belongs_to_user(&self) -> bool {
        match self.sub {
            Some(sub) => self.msg.owner == *sub,
            None => false,
        }
    }

    fn hx_trigger(&self) -> Option<&'a str> {
        match self.is_last {
            true => Some("intersect once"),
            false => None,
        }
    }

    fn hx_swap(&self) -> Option<&'a str> {
        match self.is_last {
            true => Some("afterend"),
            false => None,
        }
    }

    fn next_page(&self) -> Option<String> {
        match self.is_last {
            true => {
                let path = format!(
                    "/api/messages?limit=20&chat_id={}&end_time={}",
                    self.msg.chat_id, self.msg.timestamp
                );
                Some(path)
            }
            false => None,
        }
    }

    fn controls_handler(&self) -> Option<&str> {
        match self.belongs_to_user() {
            true => Some(
                r#"
                on mouseover remove .hidden from the first <div.message-controls/> in me
                on mouseout add .hidden to the first <div.message-controls/> in me
                "#,
            ),
            false => None,
        }
    }
}

const MESSAGE_CLASS: &str = "message-item flex items-end relative";
const MESSAGE_BUBBLE_CLASS: &str = "message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs";
const MESSAGE_TEXT_CLASS: &str =
    "message-text break-words overflow-hidden mr-2 whitespace-normal font-light";

impl Render for MessageItem<'_> {
    fn render(&self) -> Markup {
        let belongs_to_user = self.belongs_to_user();

        let message_timestamp =
            DateTime::from_timestamp(self.msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

        html! {
            div #{"m-" (self.msg._id)}
                .(MESSAGE_CLASS)
                .justify-end[belongs_to_user]
                hx-trigger=[self.hx_trigger()]
                hx-swap=[self.hx_swap()]
                hx-get=[self.next_page()]
                _=[self.controls_handler()]
            {
                @if belongs_to_user {
                    div ."message-controls hidden pb-2" {
                        (Icon::Delete(&self.msg._id))
                        (Icon::Edit) // TODO: Add edit handler
                    }

                    (Icon::Sent)

                    @if self.msg.seen {
                        (Icon::Seen)
                    }
                }

                div .(MESSAGE_BUBBLE_CLASS)
                    ."bg-blue-600 text-white ml-2"[belongs_to_user]
                    ."bg-gray-300 text-gray-600"[!belongs_to_user] {

                    p .(MESSAGE_TEXT_CLASS) lang="en" { (self.msg.text) }
                    @if let Some(timestamp) = message_timestamp {
                        span ."message-timestamp text-xs opacity-65" { (timestamp) }
                    }
                }
            }
        }
    }
}

const MAX_LEN: usize = 25;

pub fn last_message(
    lm: Option<&LastMessage>,
    chat_id: &chat::Id,
    sub: Option<&user::Sub>,
) -> Markup {
    let trim_last_message = |last_message: &LastMessage| {
        let mut text = last_message.text.clone();
        if text.len() > MAX_LEN {
            text.truncate(MAX_LEN);
            text.push_str("...");
        }
        text
    };

    html! {
        div #{"lm-"(chat_id)} ."last-message text-sm text-gray-500" {
            @if let Some(last_message) = lm {
                (trim_last_message(last_message))

                @if let Some(sender) = sub {
                    @if !last_message.seen && last_message.recipient == *sender {
                        (chat::markup::Icon::Unseen)
                    }
                } @else {
                    (chat::markup::Icon::Unseen)
                }
            }
        }
    }
}

pub enum Icon<'a> {
    Edit,
    Delete(&'a Id),
    Sent,
    Seen,
}

impl Render for Icon<'_> {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::Edit => i ."fa-pen fa-solid ml-2 text-green-700 cursor-pointer" {},
                Self::Delete(id) => {
                    i ."fa-trash-can fa-solid text-red-700 cursor-pointer"
                        hx-delete={"/api/messages/" (id)}
                        hx-target={"#m-" (id)}
                        hx-swap="outerHTML swap:200ms" {}
                },
                Self::Sent => i ."fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65" {},
                Self::Seen => i ."fa-solid fa-check absolute bottom-1 right-2.5 text-white opacity-65" {},
            }
        }
    }
}
