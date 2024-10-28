use axum::{
    extract::{Path, State},
    Extension, Form,
};
use maud::{html, Markup, Render};
use serde::Deserialize;

use crate::user::{self, model::UserInfo, service::UserService};

use super::{markup, service::ChatService, Id};

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

#[derive(Deserialize)]
pub struct CreateParams {
    sub: user::Sub,
}

pub async fn create(
    Extension(user_info): Extension<UserInfo>,
    chat_service: State<ChatService>,
    Form(params): Form<CreateParams>,
) -> crate::Result<Markup> {
    let _chat_id = chat_service.create([user_info.sub, params.sub]).await?;
    Ok(html!()) // TODO: Return chat markup
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
