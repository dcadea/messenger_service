use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::chat::model::{Chat, ChatParams};
use crate::result::Result;
use crate::state::AppState;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(find_handler))
        .route("/chats", post(create_handler))
        .with_state(state)
}

async fn find_handler(
    Query(params): Query<ChatParams>,
    state: State<AppState>,
) -> Result<Json<Vec<Chat>>> {
    match params.username {
        Some(username) => state.chat_service.find_by_username(&username).await,
        None => state.chat_service.find_all().await,
    }
    .map(Json)
}

async fn create_handler(state: State<AppState>, chat: Json<Chat>) -> Result<StatusCode> {
    state.chat_service.create(&chat).await?;
    Ok(StatusCode::CREATED)
}
