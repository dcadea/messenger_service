use crate::user::model::User;
use std::sync::Arc;

use super::repository::UserRepository;
use crate::result::Result;

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

    pub async fn find_by_sub(&self, sub: &str) -> Option<User> {
        self.repository.find_by_sub(sub).await
    }
}
