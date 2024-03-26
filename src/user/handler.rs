use warp::{Rejection, Reply};
use warp::http::StatusCode;
use warp::reply::{json, with_status};

use crate::error::model::ApiError;
use crate::user::model::{User, UserResponse};
use crate::user::repository::UserRepository;

pub async fn login_handler(user: User, user_repository: UserRepository) -> Result<impl Reply, Rejection> {
    let password = user.password();
    match user_repository.find_one(user.username()).await {
        Ok(user) => match user {
            Some(user) => {
                if user.password().eq(password) {
                    return Ok(with_status(json(&UserResponse::new(user.username())), StatusCode::OK))
                }
                Ok(with_status(json(&ApiError::new("Invalid credentials")), StatusCode::UNAUTHORIZED))
            }
            None => Ok(with_status(json(&ApiError::new("User not found")), StatusCode::NOT_FOUND))
        }
        Err(_) => Ok(with_status(json(&ApiError::new("Internal server error")), StatusCode::INTERNAL_SERVER_ERROR))
    }
}