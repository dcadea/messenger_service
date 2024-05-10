use axum::extract::State;
use axum::routing::get;
use axum::{Extension, Json, Router};
use axum_extra::extract::Query;

use crate::error::ApiError;
use crate::message::model::{Message, MessageParams};
use crate::message::service::MessageService;
use crate::result::Result;
use crate::state::AppState;
use crate::user::model::User;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", get(find_handler))
        .with_state(state)
}

async fn find_handler(
    Query(params): Query<MessageParams>,
    Extension(user): Extension<User>,
    message_service: State<MessageService>,
) -> Result<Json<Vec<Message>>> {
    match params.recipient {
        None => Err(ApiError::QueryParamRequired("recipient".to_owned())),
        Some(recipient) => {
            let mut participants = recipient.clone();
            participants.push(user.nickname);

            message_service
                .find_by_participants(&participants)
                .await
                .map(Json)
        }
    }
}
