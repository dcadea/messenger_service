use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};

use crate::result::Result;
use crate::state::AppState;
use crate::user::model::UserInfo;

use super::model::{Chat, ChatId, ChatTO};
use super::service::ChatService;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(find_handler))
        .route("/chats/:id", get(find_by_id_handler))
        .route("/chats", post(create_handler))
        .with_state(state)
}

async fn find_handler(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> Result<Json<Vec<ChatTO>>> {
    chat_service.find_for_logged_user(&user_info).await.map(Json)
}

async fn find_by_id_handler(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
    id: Path<ChatId>,
) -> Result<Json<ChatTO>> {
    chat_service.find_by_id(&id, &user_info).await.map(Json)
}

async fn create_handler(
    chat_service: State<ChatService>,
    Json(chat): Json<Chat>,
) -> Result<(StatusCode, impl IntoResponse)> {
    // TODO: check if the user is a participant of the chat
    let location = chat_service
        .create(&chat)
        .await
        .map(|chat_id| format!("/api/v1/chats/{}", chat_id))?;

    Ok((StatusCode::CREATED, [(header::LOCATION, location)]))
}
