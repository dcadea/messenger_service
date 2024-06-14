use std::sync::Arc;

use crate::chat::model::ChatId;

use super::model::{Message, MessageDto, MessageId, MessageParams};
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
    pub async fn find_by_chat_id_and_params(
        &self,
        chat_id: &ChatId,
        params: &MessageParams,
    ) -> Result<Vec<MessageDto>> {
        let limit = params.limit;
        let end_time = params.end_time;

        let result = match (limit, end_time) {
            (None, None) => self.repository.find_by_chat_id(chat_id).await?,
            (Some(limit), None) => {
                self.repository
                    .find_by_chat_id_limited(chat_id, limit)
                    .await?
            }
            (None, Some(end_time)) => {
                self.repository
                    .find_by_chat_id_before(chat_id, end_time)
                    .await?
            }
            (Some(limit), Some(end_time)) => {
                self.repository
                    .find_by_chat_id_limited_before(chat_id, limit, end_time)
                    .await?
            }
        };

        let result = result
            .iter()
            .map(|m| MessageDto::from(m))
            .collect::<Vec<_>>();

        Ok(result)
    }
}
