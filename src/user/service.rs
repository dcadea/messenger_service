use std::sync::Arc;

use crate::error::ApiError;
use crate::user::model::{User, UserResponse};
use crate::user::repository::UserRepository;

pub struct UserService {
    user_repository: Arc<UserRepository>,
}

impl UserService {
    pub fn new(user_repository: Arc<UserRepository>) -> Arc<Self> {
        Self { user_repository }.into()
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<UserResponse, ApiError> {
        return match self.user_repository.find_one(username).await {
            Some(user) => {
                if user.password().eq(password) {
                    return Ok(UserResponse::new(username));
                }

                Err(ApiError::InvalidCredentials)
            }
            None => Err(ApiError::UserNotFound),
        };
    }

    pub async fn exists(&self, username: &str) -> bool {
        self.user_repository.find_one(username).await.is_some()
    }

    pub async fn create(&self, user: &User) -> Result<UserResponse, mongodb::error::Error> {
        self.user_repository
            .insert(user)
            .await
            .map(|_| UserResponse::new(user.username()))
    }
}
