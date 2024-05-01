use crate::chat::model::Chat;
use crate::chat::repository::ChatRepository;
use crate::result::Result;
use std::sync::Arc;

pub struct ChatService {
    repository: Arc<ChatRepository>,
}

impl ChatService {
    pub fn new(repository: Arc<ChatRepository>) -> Arc<Self> {
        Self { repository }.into()
    }
}

impl ChatService {
    pub(super) async fn create(&self, chat: &Chat) -> Result<()> {
        self.repository.insert(&chat).await
    }

    pub(super) async fn find_all(&self) -> Result<Vec<Chat>> {
        self.repository.find_all().await
    }

    pub(super) async fn find_by_username(&self, username: &str) -> Result<Vec<Chat>> {
        self.repository.find_by_username(username).await
    }
}
