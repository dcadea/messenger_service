use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::{Extension, Form};
use axum_extra::extract::Query;
use maud::{Markup, Render};
use serde::Deserialize;

use crate::chat::service::ChatService;
use crate::error::Error;
use crate::user::model::UserInfo;
use crate::{chat, user};

use super::model::Message;
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
    Form(params): Form<CreateParams>,
) -> crate::Result<Markup> {
    let msg = Message::new(
        params.chat_id,
        user_info.sub.clone(),
        params.recipient,
        params.text.trim(),
    );

    let messages = message_service.create(&msg).await?;

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
    chat_service: State<ChatService>,
    message_service: State<MessageService>,
) -> crate::Result<impl IntoResponse> {
    let chat_id = params
        .chat_id
        .ok_or(Error::QueryParamRequired("chat_id".to_owned()))?;

    let logged_sub = &user_info.sub;

    chat_service.check_member(&chat_id, logged_sub).await?;

    let messages = message_service
        .find_by_chat_id_and_params(logged_sub, &chat_id, params.limit, params.end_time)
        .await?;

    let trigger_value = match params.end_time {
        Some(_) => "msg:nextPage",
        None => "msg:firstPage",
    };
    let trigger_value = HeaderValue::from_str(trigger_value).expect("invalid header value");

    let mut header_map = HeaderMap::new();
    header_map.insert("HX-Trigger", trigger_value);

    Ok((
        header_map,
        markup::MessageList::append(&messages, logged_sub).render(),
    ))
}

pub async fn delete(
    user_info: Extension<UserInfo>,
    id: Path<Id>,
    message_service: State<MessageService>,
) -> crate::Result<()> {
    message_service.delete(&user_info.sub, &id).await?;
    Ok(())
}
