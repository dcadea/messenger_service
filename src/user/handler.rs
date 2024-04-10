use std::sync::Arc;

use warp::http::StatusCode;
use warp::reply::{json, with_status};
use warp::{Rejection, Reply};

use crate::error::model::ApiError;
use crate::user::model::{User, UserResponse};
use crate::user::repository::UserRepository;

pub async fn login_handler(
    user: User,
    user_repository: Arc<UserRepository>,
) -> Result<impl Reply, Rejection> {
    let password = user.password();

    match user_repository.find_one(user.username()).await {
        Some(user) => {
            if user.password().eq(password) {
                return Ok(with_status(
                    json(&UserResponse::new(user.username())),
                    StatusCode::OK,
                ));
            }
            Ok(with_status(
                json(&ApiError::new("Invalid credentials")),
                StatusCode::UNAUTHORIZED,
            ))
        }
        None => Ok(with_status(
            json(&ApiError::new("User not found")),
            StatusCode::NOT_FOUND,
        )),
    }
}

pub async fn register_handler(
    user: User,
    user_repository: Arc<UserRepository>,
) -> Result<impl Reply, Rejection> {
    match user_repository.find_one(user.username()).await {
        Some(_) => Ok(with_status(
            json(&ApiError::new("User already exists")),
            StatusCode::CONFLICT,
        )),
        None => match user_repository.insert(&user).await {
            Ok(_) => Ok(with_status(
                json(&UserResponse::new(user.username())),
                StatusCode::CREATED,
            )),
            Err(_) => Ok(with_status(
                json(&ApiError::new("Failed to create user")),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}
