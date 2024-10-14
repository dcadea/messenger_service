use maud::{html, Markup};

use crate::user::model::UserInfo;
use crate::{chat, user};

use super::model::MessageDto;

pub fn message_input(chat_id: &chat::Id, recipient: &user::Sub) -> Markup {
    html! {
        form #message-input ."border-gray-200 flex"
        {
            input type="hidden" name="type" value="create_message" {}
            input type="hidden" name="chat_id" value=(chat_id) {}
            input type="hidden" name="recipient" value=(recipient) {}

            input ."border border-gray-300 rounded-l-md p-2 flex-1"
                type="text"
                name="text"
                placeholder="Type your message..." {}

            input ."bg-blue-600 text-white px-4 rounded-r-md"
                type="submit"
                value="Send" {}
        }
    }
}

pub(super) fn message_list(messages: &Vec<MessageDto>, user_info: &UserInfo) -> Markup {
    html! {
        div class="message-list flex flex-col" {
            @for msg in messages {
                (message_item(&msg, &user_info))
            }
        }
    }
}

pub(super) fn message_item(msg: &MessageDto, user_info: &UserInfo) -> Markup {
    let belongs_to_user = msg.owner == user_info.sub;
    let message_timestamp =
        chrono::DateTime::from_timestamp(msg.timestamp, 0).map(|dt| dt.format("%H:%M"));

    html! {
        .message-item
            id={"m-" (msg.id)}
            ."flex items-center items-baseline"
            .justify-end[belongs_to_user]
        {
            @if belongs_to_user {
                i ."fa-trash-can fa-solid text-red-700 cursor-pointer"
                    hx-delete={"/api/messages/" (msg.id)}
                    hx-target={"#m-" (msg.id)}
                    hx-swap="outerHTML" {}

                // TODO: Add edit handler
                i ."fa-pen fa-solid ml-2 text-green-700 cursor-pointer" {}
            }

            div.message-bubble
                ."flex flex-row rounded-lg p-2 mt-2 max-w-xs relative"
                ."bg-blue-600 text-white ml-2"[belongs_to_user]
                ."bg-gray-300 text-gray-600"[!belongs_to_user] {

                p.message-text ."mr-3 whitespace-normal font-light" { (msg.text) }
                @if let Some(mt) = message_timestamp {
                    span.message-timestamp .text-xs { (mt) }
                }

                @if belongs_to_user {
                    i ."fa-solid fa-check absolute bottom-1 right-1 opacity-65" {}

                    @if msg.seen {
                        i ."fa-solid fa-check absolute bottom-1 right-2.5 opacity-65" {}
                    }
                }
            }
        }
    }
}
