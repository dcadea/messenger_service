use super::model::Chat;
use super::repository::ChatRepository;
use crate::error::ApiError;
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
    pub async fn update_last_message(
        &self,
        sender: &str,
        recipient: &str,
        text: &str,
    ) -> Result<()> {
        match self
            .repository
            .find_by_sender_and_recipient(sender, recipient)
            .await
        {
            Ok(chat_id) => self.repository.update_last_message(&chat_id, text).await,
            Err(ApiError::NotFound(..)) => self
                .repository
                .insert(&Chat::new(sender, recipient, text))
                .await
                .map(|_| ()),
            Err(e) => Err(e),
        }
    }

    pub async fn find_by_sender(&self, sender: &str) -> Result<Vec<Chat>> {
        self.repository.find_by_sender(sender).await
    }
}
