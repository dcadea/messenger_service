use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::error::ApiError;
use crate::result::Result;
use crate::state::AppState;
use crate::user::model::User;

pub fn auth_router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
        .with_state(state)
}

async fn login_handler(state: State<AppState>, user: Json<User>) -> Result<Json<Value>> {
    state
        .user_service
        .matches(&user.username, &user.password)
        .await?;
    Ok(Json::from(json!({"username": user.username})))
}

async fn register_handler(state: State<AppState>, user: Json<User>) -> impl IntoResponse {
    if state.user_service.exists(&user.username).await {
        return Err(ApiError::UserAlreadyExists);
    }

    state.user_service.create(&user).await?;
    Ok(StatusCode::CREATED)
}
