use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Form,
};
use maud::{Markup, Render};
use messenger_service::markup::Wrappable;
use serde::Deserialize;

use crate::user::{self, model::UserInfo, service::UserService};

use super::{markup, service::ChatService, Id};

pub async fn home(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> crate::Result<Wrappable> {
    let chats = chat_service.find_all(&user_info).await?;
    Ok(Wrappable::new(markup::ChatList::new(&user_info, &chats)).with_sse())
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
    Extension(logged_user): Extension<UserInfo>,
    chat_service: State<ChatService>,
    user_service: State<UserService>,
    Form(params): Form<CreateParams>,
) -> crate::Result<Markup> {
    let recipient = &params.sub;
    let chat = chat_service.create(&logged_user, recipient).await?;
    let recipient = user_service.find_user_info(recipient).await?;

    Ok(markup::ActiveChat::new(&chat.id, &recipient).render())
}

pub async fn delete(
    chat_id: Path<Id>,
    logged_user: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> crate::Result<impl IntoResponse> {
    chat_service.delete(&chat_id, &logged_user).await?;

    Ok([("HX-Redirect", "/")])
}

pub async fn open_chat(
    chat_id: Path<Id>,
    logged_user: Extension<UserInfo>,
    chat_service: State<ChatService>,
    user_service: State<UserService>,
) -> crate::Result<Markup> {
    let chat = chat_service.find_by_id(&chat_id, &logged_user).await?;
    let recipient = user_service.find_user_info(&chat.recipient).await?;

    Ok(markup::ActiveChat::new(&chat.id, &recipient).render())
}
