use std::sync::Arc;

use crate::user::repository::UserRepository;

pub struct UserService {
    repository: Arc<UserRepository>,
}

impl UserService {
    pub fn new(repository: Arc<UserRepository>) -> Arc<Self> {
        Self {
            repository,
        }
        .into()
    }
}

impl UserService {
    pub async fn exists(&self, nickname: &str) -> bool {
        self.repository.find_one(nickname).await.is_some()
    }
}
