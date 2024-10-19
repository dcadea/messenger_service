use axum::extract::{Path, State};
use axum::{Extension, Form};
use axum_extra::extract::Query;
use maud::Markup;
use serde::Deserialize;

use crate::chat;
use crate::chat::service::ChatService;
use crate::error::Error;
use crate::user::model::UserInfo;

use super::model::{Message, MessageDto, MessageRequest};
use super::service::MessageService;
use super::{markup, Id};

#[derive(Deserialize)]
pub struct Params {
    chat_id: Option<chat::Id>,
    end_time: Option<i64>,
    limit: Option<usize>,
}

pub async fn find_all(
    user_info: Extension<UserInfo>,
    params: Query<Params>,
    chat_service: State<ChatService>,
    message_service: State<MessageService>,
) -> crate::Result<Markup> {
    let chat_id = params
        .chat_id
        .ok_or(Error::QueryParamRequired("chat_id".to_owned()))?;

    chat_service.check_member(&chat_id, &user_info.sub).await?;

    let messages = message_service
        .find_by_chat_id_and_params(&chat_id, params.limit, params.end_time)
        .await?;

    Ok(markup::message_list(&messages, &user_info))
}

pub async fn find_one(
    id: Path<Id>,
    user_info: Extension<UserInfo>,
    message_service: State<MessageService>,
) -> crate::Result<Markup> {
    // TODO: validate user is a member of the chat

    let msg = message_service.find_by_id(&id).await?;

    Ok(markup::message_item(&msg, &user_info))
}

pub async fn create(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    message_service: State<MessageService>,
    req: Form<MessageRequest>,
) -> crate::Result<Markup> {
    chat_service
        .check_members(&req.chat_id, [&user_info.sub, &req.recipient])
        .await?;

    let msg = Message::new(
        req.chat_id,
        user_info.sub.clone(),
        req.recipient.clone(),
        &req.text,
    );

    let msg = message_service.create(&msg).await?;
    chat_service.update_last_message(&msg).await?;

    Ok(markup::message_item(&MessageDto::from(msg), &user_info))
}

pub async fn delete(id: Path<Id>, message_service: State<MessageService>) -> crate::Result<()> {
    // TODO: validate user is a member of the chat

    message_service.delete(&id).await?;

    Ok(())
}
