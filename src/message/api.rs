use crate::auth::model::UserInfo;
use axum::extract::State;
use axum::routing::get;
use axum::{Extension, Json, Router};
use axum_extra::extract::Query;

use super::model::{Message, MessageParams};
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
    Query(params): Query<MessageParams>,
    Extension(user_info): Extension<UserInfo>,
    message_service: State<MessageService>,
) -> Result<Json<Vec<Message>>> {
    match params.companion {
        None => Err(ApiError::QueryParamRequired("companion".to_owned())),
        Some(companion) => {
            let mut participants = companion.clone();
            participants.push(user_info.nickname);

            message_service
                .find_by_participants(&participants)
                .await
                .map(Json)
        }
    }
}
