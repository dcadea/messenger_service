use axum::extract::{Path, State};
use axum::Extension;
use axum_extra::extract::Query;
use maud::Markup;
use serde::Deserialize;

use crate::chat;
use crate::chat::service::ChatService;
use crate::error::Error;
use crate::user::model::UserInfo;

use super::service::MessageService;
use super::{markup, Id};

#[derive(Deserialize)]
pub(super) struct Params {
    chat_id: Option<chat::Id>,
    end_time: Option<i64>,
    limit: Option<usize>,
}

pub(super) async fn find_all(
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

pub(super) async fn find_one(
    id: Path<Id>,
    user_info: Extension<UserInfo>,
    message_service: State<MessageService>,
) -> crate::Result<Markup> {
    // TODO: chat_service.check_member(&chat_id, &user_info.sub).await?;

    let msg = message_service.find_by_id(&id).await?;

    Ok(markup::message_item(&msg, &user_info))
}

pub async fn delete(id: Path<Id>, message_service: State<MessageService>) -> crate::Result<()> {
    // TODO: chat_service.check_member(&chat_id, &user_info.sub).await?;

    message_service.delete(&id).await?;

    Ok(())
}
