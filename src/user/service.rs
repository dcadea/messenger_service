use crate::user::model::User;
use std::sync::Arc;

use crate::result::Result;
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
    pub(crate) async fn create(&self, user: &User) -> Result<()> {
        self.repository.insert(user).await
    }

    pub(crate) async fn find_by_sub(&self, sub: &str) -> Option<User> {
        self.repository.find_by_sub(sub).await
    }
}
