use crate::auth::model::UserInfo;
use axum::extract::State;
use axum::routing::get;
use axum::{Extension, Json, Router};

use super::model::Chat;
use super::service::ChatService;
use crate::result::Result;
use crate::state::AppState;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats", get(find_handler))
        .with_state(state)
}

async fn find_handler(
    user_info: Extension<UserInfo>,
    chat_service: State<ChatService>,
) -> Result<Json<Vec<Chat>>> {
    chat_service.find_by_sender(&user_info.sub).await.map(Json)
}
