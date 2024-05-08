use crate::chat::model::Chat;
use crate::chat::repository::ChatRepository;
use crate::result::Result;
use std::sync::Arc;

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<ChatRepository>,
}

impl ChatService {
    pub fn new(repository: ChatRepository) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
}

impl ChatService {
    pub(super) async fn create(&self, chat: &Chat) -> Result<()> {
        self.repository.insert(chat).await
    }

    pub(super) async fn find_by_nickname(&self, nickname: &str) -> Result<Vec<Chat>> {
        self.repository.find_by_nickname(nickname).await
    }
}
