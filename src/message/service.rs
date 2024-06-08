use std::sync::Arc;

use crate::chat::model::ChatId;

use super::model::{Message, MessageDto, MessageId};
use super::repository::MessageRepository;
use super::Result;

#[derive(Clone)]
pub struct MessageService {
    repository: Arc<MessageRepository>,
}

impl MessageService {
    pub fn new(repository: MessageRepository) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
}

impl MessageService {
    pub async fn create(&self, message: &Message) -> Result<Message> {
        self.repository
            .insert(message)
            .await
            .map(|id| message.with_id(id))
    }

    pub async fn find_by_id(&self, id: MessageId) -> Result<Message> {
        self.repository.find_by_id(id).await
    }

    pub async fn update(&self, id: &MessageId, text: &str) -> Result<()> {
        self.repository.update(&id, text).await
    }

    pub async fn delete(&self, id: &MessageId) -> Result<()> {
        self.repository.delete(&id).await
    }

    pub async fn mark_as_seen(&self, id: &MessageId) -> Result<()> {
        self.repository.mark_as_seen(&id).await
    }
}

impl MessageService {
    pub async fn find_by_chat_id(&self, chat_id: &ChatId) -> Result<Vec<MessageDto>> {
        let result = self
            .repository
            .find_by_chat_id(&chat_id)
            .await
            .iter()
            .flatten()
            .map(|m| MessageDto::from(m))
            .collect::<Vec<_>>();

        Ok(result)
    }
}
