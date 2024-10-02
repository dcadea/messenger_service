use axum::extract::{Path, State};
use axum::routing::{delete, get};
use axum::{Extension, Router};
use axum_extra::extract::Query;
use maud::{html, Markup, Render};

use crate::chat::model::ChatId;
use crate::chat::service::ChatService;
use crate::error::Error;
use crate::result::Result;
use crate::state::AppState;
use crate::user::model::{Sub, UserInfo};

use super::model::{MessageDto, MessageId, MessageParams};
use super::service::MessageService;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", get(find_all))
        .route("/messages/:id", get(find_one))
        .route("/messages/:id", delete(delete_one))
        .with_state(state)
}

async fn find_all(
    user_info: Extension<UserInfo>,
    params: Query<MessageParams>,
    chat_service: State<ChatService>,
    message_service: State<MessageService>,
) -> Result<Markup> {
    let chat_id = params
        .chat_id
        .ok_or(Error::QueryParamRequired("chat_id".to_owned()))?;

    chat_service.check_member(&chat_id, &user_info.sub).await?;

    let messages = message_service
        .find_by_chat_id_and_params(&chat_id, &params)
        .await?;

    Ok(html! {
        div class="message-list flex flex-col" {
            @for msg in messages {
                (message_item(&msg, &user_info))
            }
        }
    })
}

async fn find_one(
    id: Path<MessageId>,
    user_info: Extension<UserInfo>,
    message_service: State<MessageService>,
) -> Result<Markup> {
    // TODO: chat_service.check_member(&chat_id, &user_info.sub).await?;

    let msg = message_service.find_by_id(&id).await?;

    Ok(message_item(&msg, &user_info))
}

async fn delete_one(id: Path<MessageId>, message_service: State<MessageService>) -> Result<()> {
    // TODO: chat_service.check_member(&chat_id, &user_info.sub).await?;

    message_service.delete(&id).await?;

    Ok(())
}

pub(crate) fn message_input(chat_id: &ChatId, recipient: &Sub) -> Markup {
    html! {
        form #message-input
            ws-send
            ."border-gray-200 flex"
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

fn message_item(msg: &MessageDto, user_info: &UserInfo) -> Markup {
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
