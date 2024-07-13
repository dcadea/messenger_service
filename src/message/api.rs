use axum::extract::State;
use axum::routing::get;
use axum::{Extension, Json, Router};
use axum_extra::extract::Query;

use crate::chat::service::ChatService;
use crate::error::ApiError;
use crate::result::Result;
use crate::state::AppState;
use crate::user::model::UserInfo;

use super::model::{MessageDto, MessageParams};
use super::service::MessageService;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", get(find_handler))
        .with_state(state)
}

async fn find_handler(
    user_info: Extension<UserInfo>,
    params: Query<MessageParams>,
    chat_service: State<ChatService>,
    message_service: State<MessageService>,
) -> Result<Json<Vec<MessageDto>>> {
    let chat_id = params
        .chat_id
        .ok_or(ApiError::QueryParamRequired("chat_id".to_owned()))?;

    chat_service.check_member(chat_id, &user_info.sub).await?;

    let result = message_service
        .find_by_chat_id_and_params(&chat_id, &params)
        .await?;

    Ok(Json(result))
}
