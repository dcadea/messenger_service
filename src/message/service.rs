use std::sync::Arc;

use crate::message::model::Message;
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
    pub async fn create(&self, message: &Message) -> Result<()> {
        self.repository.insert(message).await
    }

    pub async fn find_by_participants(&self, participants: &Vec<String>) -> Result<Vec<Message>> {
        self.repository.find_by_participants(participants).await
    }
}
