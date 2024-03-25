use warp::{Rejection, Reply};
use warp::http::StatusCode;
use warp::reply::json;

use crate::error::model::ApiError;
use crate::user::model::{User, UserResponse};
use crate::user::repository::UserRepository;

pub async fn login_handler(user: User, user_repository: UserRepository) -> Result<impl Reply, Rejection> {
    let password = user.password();

    let result = user_repository.find_one(user.username()).await;
    match result {
        Ok(user) => match user {
            Some(user) => {
                if user.password().eq(password) {
                    let response = UserResponse::new(user.username());
                    Ok(warp::reply::with_status(json(&response), StatusCode::OK))
                } else {
                    let error = ApiError::new(401, "Invalid credentials");
                    Ok(warp::reply::with_status(json(&error), StatusCode::UNAUTHORIZED))
                }
            }
            None => {
                let error = ApiError::new(404, "User not found");
                Ok(warp::reply::with_status(json(&error), StatusCode::NOT_FOUND))
            }
        }
        Err(_) => {
            let error = ApiError::new(500, "Internal server error");
            Ok(warp::reply::with_status(json(&error), StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}