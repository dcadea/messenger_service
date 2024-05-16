use crate::auth::model::UserInfo;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};

use crate::chat::model::{Chat, ChatRequest};
use crate::chat::service::ChatService;
use crate::result::Result;
use crate::state::AppState;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(find_handler))
        .route("/chats", post(create_handler))
        .with_state(state)
}

async fn find_handler(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> Result<Json<Vec<Chat>>> {
    chat_service
        .find_by_sender(&user_info.nickname)
        .await
        .map(Json)
}

async fn create_handler(
    chat_service: State<ChatService>,
    Extension(user_info): Extension<UserInfo>,
    Json(chat_request): Json<ChatRequest>,
) -> Result<StatusCode> {
    chat_service
        .create(&Chat::from_request(&user_info.nickname, chat_request))
        .await?;
    Ok(StatusCode::CREATED)
}
