use std::sync::Arc;

use crate::error::ApiError;
use crate::message::model::Message;
use crate::result::Result;
use crate::user::model::UserSub;

use super::model::{Chat, ChatId, Members};
use super::repository::ChatRepository;

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
    pub async fn create(&self, chat: &Chat) -> Result<ChatId> {
        self.repository.insert(chat).await
    }

    pub async fn update_last_message(&self, message: &Message) -> Result<()> {
        let members = Members::new(message.owner.clone(), message.recipient.clone());

        match self.repository.find_id_by_members(&members).await {
            Ok(chat_id) => {
                self.repository
                    .update_last_message(&chat_id, &message.text)
                    .await
            }
            Err(ApiError::NotFound(..)) => self
                .repository
                .insert(&Chat::new(members, &message.text))
                .await
                .map(|_| ()),
            Err(e) => Err(e),
        }
    }

    pub async fn find_by_sub(&self, sub: &UserSub) -> Result<Vec<Chat>> {
        self.repository.find_by_sub(sub).await
    }
}
