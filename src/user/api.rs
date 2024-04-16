use axum::extract::State;
use axum::response::Result;
use axum::routing::post;
use axum::{Json, Router};

use crate::error::ApiError;
use crate::state::AppState;
use crate::user::model::{User, UserResponse};

pub fn router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
        .with_state(state)
}

async fn login_handler(
    state: State<AppState>,
    user: Json<User>,
) -> Result<Json<UserResponse>, ApiError> {
    state
        .user_service
        .login(user.username(), user.password())
        .await
        .map(Json)
}

async fn register_handler(
    state: State<AppState>,
    user: Json<User>,
) -> Result<Json<UserResponse>, ApiError> {
    if state.user_service.exists(user.username()).await {
        return Err(ApiError::UserAlreadyExists);
    }

    let created = state.user_service.create(&user).await?;

    // TODO: return 201 Created
    Ok(Json(created))
}
