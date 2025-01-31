use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Extension, Form};
use axum_extra::extract::Query;
use maud::{Markup, Render};
use serde::Deserialize;

use crate::chat::service::{ChatService, ChatValidator};
use crate::error::Error;
use crate::user::model::UserInfo;
use crate::{chat, user};

use super::model::{LastMessage, Message};
use super::service::MessageService;
use super::{markup, Id};

#[derive(Deserialize)]
pub struct CreateParams {
    chat_id: chat::Id,
    recipient: user::Sub,
    text: String,
}

pub async fn create(
    user_info: Extension<UserInfo>,
    message_service: State<MessageService>,
    chat_service: State<ChatService>,
    Form(params): Form<CreateParams>,
) -> crate::Result<Markup> {
    let msg = Message::new(
        params.chat_id,
        user_info.sub.clone(),
        params.recipient,
        params.text.trim(),
    );

    let messages = message_service.create(&msg).await?;

    if let Some(last_msg) = messages.last() {
        let last_message = LastMessage::from(last_msg);
        chat_service
            .update_last_message(&last_msg.chat_id, Some(&last_message))
            .await?;
    }

    Ok(markup::MessageList::prepend(&messages, &user_info.sub).render())
}

#[derive(Deserialize)]
pub struct FindAllParams {
    chat_id: Option<chat::Id>,
    end_time: Option<i64>,
    limit: Option<usize>,
}

pub async fn find_all(
    user_info: Extension<UserInfo>,
    Query(params): Query<FindAllParams>,
    chat_validator: State<ChatValidator>,
    chat_service: State<ChatService>,
    message_service: State<MessageService>,
) -> crate::Result<impl IntoResponse> {
    let chat_id = params
        .chat_id
        .ok_or(Error::QueryParamRequired("chat_id".to_owned()))?;

    let logged_sub = &user_info.sub;

    chat_validator.check_member(&chat_id, logged_sub).await?;

    let (messages, seen_qty) = message_service
        .find_by_chat_id_and_params(logged_sub, &chat_id, params.limit, params.end_time)
        .await?;

    if seen_qty > 0 {
        chat_service.mark_as_seen(&chat_id).await?;
    }

    Ok(markup::MessageList::append(&messages, logged_sub).render())
}

pub async fn delete(
    user_info: Extension<UserInfo>,
    Path(id): Path<Id>,
    message_service: State<MessageService>,
    chat_service: State<ChatService>,
) -> crate::Result<()> {
    if let Some(deleted_msg) = message_service.delete(&user_info.sub, &id).await? {
        let is_last = chat_service.is_last_message(&deleted_msg).await?;
        if is_last {
            let chat_id = &deleted_msg.chat_id;
            let last_message = message_service
                .find_most_recent(chat_id)
                .await?
                .map(|msg| LastMessage::from(&msg));

            chat_service
                .update_last_message(chat_id, last_message.as_ref())
                .await?;
        }

        return Ok(());
    }

    Err(super::Error::NotFound(Some(id)))?
}
