use axum::{
    extract::{Path, State},
    Extension, Json,
};
use maud::{Markup, Render};

use crate::user::{model::UserInfo, service::UserService};

use super::{markup, model::ChatRequest, service::ChatService, Id};

pub async fn create(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    Json(chat_request): Json<ChatRequest>,
) -> crate::Result<()> {
    let _ = chat_service.create(&chat_request, &user_info).await?;

    Ok(())
}

pub async fn find_all(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> crate::Result<Markup> {
    let chats = chat_service.find_all(&user_info).await?;
    Ok(markup::chat_list(&chats))
}

pub async fn find_one(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    Path(id): Path<Id>,
) -> crate::Result<Markup> {
    let chat = chat_service.find_by_id(&id, &user_info).await?;
    Ok(chat.render())
}

pub async fn open_chat(
    chat_id: Path<Id>,
    logged_user: Extension<UserInfo>,
    chat_service: State<ChatService>,
    user_service: State<UserService>,
) -> crate::Result<Markup> {
    let chat = chat_service.find_by_id(&chat_id, &logged_user).await?;
    let recipient = user_service.find_user_info(&chat.recipient).await?;

    Ok(markup::active_chat(&chat.id, &recipient))
}
