use axum::extract::State;
use axum::response::{ErrorResponse, Result};
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

async fn login_handler(state: State<AppState>, user: Json<User>) -> Result<Json<UserResponse>> {
    let password = user.password();

    match state.user_repository.find_one(user.username()).await {
        Some(u) => {
            if u.password().eq(password) {
                return Ok(Json(UserResponse::new(user.username())));
            }

            return Err(ErrorResponse::from(ApiError::InvalidCredentials));
        }
        None => Err(ErrorResponse::from(ApiError::UserNotFound)),
    }
}

async fn register_handler(state: State<AppState>, user: Json<User>) -> Result<Json<UserResponse>> {
    match state.user_repository.find_one(user.username()).await {
        Some(_) => Err(ErrorResponse::from(ApiError::UserAlreadyExists)),
        None => {
            match state.user_repository.insert(&user).await {
                // TODO: return 201 Created
                Ok(_) => Ok(Json(UserResponse::new(user.username()))),
                Err(_) => Err(ErrorResponse::from(ApiError::InternalServerError)),
            }
        }
    }
}
