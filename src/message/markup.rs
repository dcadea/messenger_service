use chrono::DateTime;
use maud::{html, Markup};

use crate::{chat, user};

use super::model::MessageDto;

pub fn message_input(chat_id: &chat::Id, recipient: &user::Sub) -> Markup {
    html! {
        form id="message-input"
            class="border-gray-200 flex"
            ws-send
        {
            input type="hidden" name="type" value="create_message" {}
            input type="hidden" name="chat_id" value=(chat_id.0) {}
            input type="hidden" name="recipient" value=(recipient) {}

            input class="border border-gray-300 rounded-l-md p-2 flex-1"
                type="text"
                name="text"
                placeholder="Type your message..." {}

            input class="bg-blue-600 text-white px-4 rounded-r-md"
                type="submit"
                value="Send" {}
        }
    }
}

pub fn message_list(messages: &[MessageDto], logged_sub: &user::Sub) -> Markup {
    html! {
        @for i in 0..messages.len() {
            @if i == messages.len() - 1 {
                (last_message_item(&messages[i], logged_sub))
            } @else {
                (message_item(&messages[i], logged_sub))
            }
        }
    }
}

pub fn message_item(msg: &MessageDto, sub: &user::Sub) -> Markup {
    let belongs_to_user = msg.owner == *sub;

    html! {
        div id={"m-" (msg.id.0)}
            ."message-item flex items-center items-baseline"
            .justify-end[belongs_to_user]
        {
            (message_bubble(msg, belongs_to_user))
        }
    }
}

/// Renders the last message in the list with a trigger to load more messages
fn last_message_item(msg: &MessageDto, sub: &user::Sub) -> Markup {
    let belongs_to_user = msg.owner == *sub;

    html! {
        div id={"m-" (msg.id.0)}

            ."message-item flex items-center items-baseline"
            .justify-end[belongs_to_user]
            // FIXME: new messages are pushed into view
            // which results in a loop of requests
            // hx-trigger="intersect once"
            hx-trigger="click"
            hx-swap="afterend"
            hx-get={ "/api/messages?limit=14&chat_id=" (msg.chat_id.0) "&end_time=" (msg.timestamp) }
        {
            (message_bubble(msg, belongs_to_user))
        }
    }
}

fn message_bubble(msg: &MessageDto, belongs_to_user: bool) -> Markup {
    let message_timestamp = DateTime::from_timestamp(msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

    html! {
        @if belongs_to_user {
            i class="fa-trash-can fa-solid text-red-700 cursor-pointer"
                hx-delete={"/api/messages/" (msg.id.0)}
                hx-target={"#m-" (msg.id.0)}
                hx-swap="outerHTML" {}

            // TODO: Add edit handler
            i class="fa-pen fa-solid ml-2 text-green-700 cursor-pointer" {}
        }

        div ."message-bubble flex flex-row rounded-lg p-2 mt-2 max-w-xs relative"
            ."bg-blue-600 text-white ml-2"[belongs_to_user]
            ."bg-gray-300 text-gray-600"[!belongs_to_user] {

            p class="message-text mr-3 whitespace-normal font-light" { (msg.text) }
            @if let Some(mt) = message_timestamp {
                span class="message-timestamp text-xs" { (mt) }
            }

            @if belongs_to_user {
                i class="fa-solid fa-check absolute bottom-1 right-1 opacity-65" {}

                @if msg.seen {
                    i class="fa-solid fa-check absolute bottom-1 right-2.5 opacity-65" {}
                }
            }
        }
    }
}
