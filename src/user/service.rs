use crate::user::model::{User, UserInfo};
use std::sync::Arc;

use super::repository::UserRepository;
use super::Result;

#[derive(Clone)]
pub struct UserService {
    repository: Arc<UserRepository>,
}

impl UserService {
    pub fn new(repository: UserRepository) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
}

impl UserService {
    pub async fn create(&self, user: &User) -> Result<()> {
        self.repository.insert(user).await
    }

    pub async fn find_user_info(&self, sub: &str) -> Result<UserInfo> {
        self.repository.find_by_sub(sub).await.map(|u| u.into())
    }
}
