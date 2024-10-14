use axum::{extract::State, Extension, Json};

use crate::user::model::UserInfo;

use super::{model::ChatRequest, service::ChatService};

pub async fn create(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    Json(chat_request): Json<ChatRequest>,
) -> crate::Result<()> {
    let _ = chat_service.create(&chat_request, &user_info).await?;

    Ok(())
}
