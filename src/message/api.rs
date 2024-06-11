use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;

use crate::error::ApiError;
use crate::result::Result;
use crate::state::AppState;

use super::model::{MessageDto, MessageParams};
use super::service::MessageService;

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
    let chat_id = params
        .chat_id
        .ok_or(ApiError::QueryParamRequired("chat_id".to_owned()))?;
    let result = message_service.find_by_chat_id(&chat_id).await?;
    Ok(Json(result))
}
