use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};

use crate::chat::model::{Chat, ChatRequest};
use crate::chat::service::ChatService;
use crate::result::Result;
use crate::state::AppState;
use crate::user::model::User;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(find_handler))
        .route("/chats", post(create_handler))
        .with_state(state)
}

async fn find_handler(user: Extension<User>, chat_service: State<Arc<ChatService>>) -> Result<Json<Vec<Chat>>> {
    chat_service
        .find_by_nickname(&user.nickname)
        .await
        .map(Json)
}

async fn create_handler(
    chat_service: State<Arc<ChatService>>,
    Extension(user): Extension<User>,
    Json(chat_request): Json<ChatRequest>,
) -> Result<StatusCode> {
    chat_service
        .create(&Chat::from_request(&user.nickname, chat_request))
        .await?;
    Ok(StatusCode::CREATED)
}
