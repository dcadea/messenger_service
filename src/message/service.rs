use std::sync::Arc;

use crate::message::model::{Message, MessageId};
use crate::message::repository::MessageRepository;
use crate::result::Result;

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
    pub async fn create(&self, message: &Message) -> Result<MessageId> {
        self.repository.insert(message).await
    }

    pub async fn find_by_id(&self, id: &MessageId) -> Result<Message> {
        self.repository.find_by_id(&id).await
    }

    pub async fn find_by_participants(&self, participants: &Vec<String>) -> Result<Vec<Message>> {
        self.repository.find_by_participants(participants).await
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
