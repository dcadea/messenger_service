use std::sync::Arc;

use crate::error::ApiError;
use crate::result::Result;
use crate::user::model::User;
use crate::user::repository::UserRepository;

pub struct UserService {
    repository: Arc<UserRepository>,
}

impl UserService {
    pub fn new(repository: Arc<UserRepository>) -> Arc<Self> {
        Self { repository }.into()
    }
}

impl UserService {
    pub async fn create(&self, user: &User) -> Result<()> {
        self.repository.insert(user).await
    }

    pub async fn matches(&self, username: &str, password: &str) -> Result<()> {
        match self.repository.find_one(username).await {
            Some(user) => {
                if user.password.eq(password) {
                    return Ok(());
                }

                Err(ApiError::InvalidCredentials)
            }
            None => Err(ApiError::UserNotFound),
        }
    }

    pub async fn exists(&self, username: &str) -> bool {
        self.repository.find_one(username).await.is_some()
    }
}
