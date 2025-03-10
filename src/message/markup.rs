use chrono::DateTime;
use maud::{Markup, Render, html};
use messenger_service::markup::Id;

use crate::{chat, message, user};

use super::model::{LastMessage, Message};

pub struct InputBlank<'a> {
    pub chat_id: &'a chat::Id,
    pub recipient: &'a user::Sub,
}

impl<'a> InputBlank<'a> {
    pub fn new(chat_id: &'a chat::Id, recipient: &'a user::Sub) -> Self {
        Self { chat_id, recipient }
    }
}

const MESSAGE_INPUT_ID: &str = "message-input";
pub const MESSAGE_INPUT_TARGET: &str = "#message-input";

pub const MESSAGE_LIST_ID: &str = "message-list";
const MESSAGE_LIST_TARGET: &str = "#message-list";

impl Render for InputBlank<'_> {
    fn render(&self) -> Markup {
        let send_message_handler = format!(
            r#"
                on htmx:afterRequest reset() me
                on htmx:afterRequest go to the bottom of the {}
            "#,
            MESSAGE_LIST_TARGET
        );

        html! {
            form #(MESSAGE_INPUT_ID) ."border-gray-200 flex"
                hx-post="/api/messages"
                hx-target=(MESSAGE_LIST_TARGET)
                hx-swap="afterbegin"
                _=(send_message_handler)
            {
                input type="hidden" name="chat_id" value=(self.chat_id) {}
                input type="hidden" name="recipient" value=(self.recipient) {}
                (InputText(None))
                (SendButton)
            }
        }
    }
}

pub struct InputEdit<'a> {
    id: &'a message::Id,
    old_text: &'a str,
}

impl<'a> InputEdit<'a> {
    pub fn new(id: &'a message::Id, old_text: &'a str) -> Self {
        Self { id, old_text }
    }
}

impl Render for InputEdit<'_> {
    fn render(&self) -> Markup {
        html! {
            form #(MESSAGE_INPUT_ID) ."border-gray-200 flex"
                hx-put="/api/messages"
                hx-target=(self.id.target())
                hx-swap="outerHTML"
            {
                input type="hidden" name="message_id" value=(self.id) {}
                (InputText(Some(self.old_text)))
                (SendButton)
            }

        }
    }
}

struct InputText<'a>(Option<&'a str>);

impl Render for InputText<'_> {
    fn render(&self) -> Markup {
        html! {
            input ."border border-gray-300 rounded-l-md p-2 flex-1 focus:outline-none"
                type="text"
                name="text"
                value=[self.0]
                placeholder="Type your message..."
                autocomplete="off"
                hx-disabled-elt="this"
                _="on keyup if the event's key is 'Escape' set value of me to ''" {}
        }
    }
}

struct SendButton;

impl Render for SendButton {
    fn render(&self) -> Markup {
        html! {
            input ."bg-blue-600 text-white px-4 rounded-r-md cursor-pointer hover:bg-blue-700"
                hx-disabled-elt="this"
                type="submit"
                value="Send" {}
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

        let msg_timestamp =
            DateTime::from_timestamp(self.msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

        html! {
            div #(self.msg._id.attr())
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
                        (Icon::Edit(&self.msg))
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
                    @if let Some(timestamp) = msg_timestamp {
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
    let trim = |lm: &LastMessage| {
        let mut text = lm.text.clone();
        if text.len() > MAX_LEN {
            text.truncate(MAX_LEN);
            text.push_str("...");
        }
        text
    };

    html! {
        div #{"lm-"(chat_id)} ."last-message text-sm text-gray-500" {
            @if let Some(last_msg) = lm {
                (trim(last_msg))

                @if let Some(sender) = sub {
                    @if !last_msg.seen && last_msg.recipient == *sender {
                        (chat::markup::Icon::Unseen)
                    }
                } @else {
                    (chat::markup::Icon::Unseen)
                }
            }
        }
    }
}

impl messenger_service::markup::Id for message::Id {
    fn attr(&self) -> String {
        format!("m-{}", self.0)
    }

    fn target(&self) -> String {
        format!("#m-{}", self.0)
    }
}

pub enum Icon<'a> {
    Edit(&'a Message),
    Delete(&'a message::Id),
    Sent,
    Seen,
}

impl Render for Icon<'_> {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::Edit(msg) =>{
                    i ."fa-pen fa-solid ml-2 text-green-700 cursor-pointer"
                        hx-get={"/templates/messages/input/edit?message_id=" (msg._id)}
                        hx-target=(MESSAGE_INPUT_TARGET)
                        hx-swap="outerHTML" {}
                },
                Self::Delete(id) => {
                    i ."fa-trash-can fa-solid text-red-700 cursor-pointer"
                        hx-delete={"/api/messages/" (id)}
                        hx-target=(id.target())
                        hx-swap="outerHTML swap:200ms" {}
                },
                Self::Sent => i ."fa-solid fa-check absolute bottom-1 right-1 text-white opacity-65" {},
                Self::Seen => i ."fa-solid fa-check absolute bottom-1 right-2.5 text-white opacity-65" {},
            }
        }
    }
}
