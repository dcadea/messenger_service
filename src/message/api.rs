use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;

use super::model::{MessageDto, MessageParams};
use super::service::MessageService;
use crate::error::ApiError;
use crate::result::Result;
use crate::state::AppState;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", get(find_handler))
        .with_state(state)
}

async fn find_handler(
    params: Query<MessageParams>,
    message_service: State<MessageService>,
) -> Result<Json<Vec<MessageDto>>> {
    // TODO: check if logged in user is a participant of the chat

    match &params.chat_id {
        None => Err(ApiError::QueryParamRequired("chat_id".to_owned())),
        Some(chat_id) => {
            let result = message_service.find_by_chat_id(chat_id).await?;
            Ok(Json(result))
        }
    }
}
