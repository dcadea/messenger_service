use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::Query;

use crate::error::ApiError;
use crate::result::Result;
use crate::state::AppState;

use super::model::{UserInfo, UserParams};
use super::service::UserService;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/users", get(find_handler))
        .with_state(state)
}

async fn find_handler(
    params: Query<UserParams>,
    user_service: State<UserService>,
) -> Result<Json<UserInfo>> {
    match &params.sub {
        None => Err(ApiError::QueryParamRequired("sub".to_owned())),
        Some(sub) => {
            let result = user_service.find_user_info(sub).await?;
            Ok(Json(result))
        }
    }
}
