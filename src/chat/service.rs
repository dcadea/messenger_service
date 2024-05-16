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
    pub async fn create(&self, chat: &Chat) -> Result<()> {
        self.repository.insert(chat).await
    }

    pub async fn find_by_sender(&self, sender: &str) -> Result<Vec<Chat>> {
        self.repository.find_by_sender(sender).await
    }
}
