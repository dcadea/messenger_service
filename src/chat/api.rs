use crate::chat::model::Chat;
use crate::result::Result;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

pub fn router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats/:username", get(chat_handler))
        .with_state(state)
}

async fn chat_handler(
    Path(username): Path<String>,
    state: State<AppState>,
) -> Result<Json<Vec<Chat>>> {
    state
        .chat_service
        .find_by_username(&username)
        .await
        .map(Json)
}
