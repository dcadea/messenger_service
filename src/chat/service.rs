use super::error::ChatError;
use std::sync::Arc;

use super::Result;
use crate::message::model::Message;
use crate::user::model::UserInfo;

use super::model::{Chat, ChatDto, ChatId, ChatRequest, Members};
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
    pub async fn create(&self, chat_request: &ChatRequest) -> Result<ChatId> {
        self.repository.insert(&chat_request.into()).await
    }

    pub async fn update_last_message(&self, message: &Message) -> Result<()> {
        let members = Members::new(message.owner.clone(), message.recipient.clone());

        match self.repository.find_id_by_members(&members).await {
            Ok(chat_id) => {
                self.repository
                    .update_last_message(&chat_id, &message.text)
                    .await
            }
            Err(ChatError::NotFound(..)) => self
                .repository
                .insert(&Chat::new(members, &message.text))
                .await
                .map(|_| ()),
            Err(e) => Err(e),
        }
    }

    pub async fn find_by_id(&self, id: ChatId, user_info: &UserInfo) -> Result<ChatDto> {
        self.repository
            .find_by_id(id)
            .await
            .map(|chat| Self::map_to_transfer_object(chat, &user_info))
    }

    pub async fn find_for_logged_user(&self, user_info: &UserInfo) -> Result<Vec<ChatDto>> {
        self.repository
            .find_by_sub(&user_info.sub)
            .await
            .map(|chats| {
                chats
                    .into_iter()
                    .map(|chat| Self::map_to_transfer_object(chat, &user_info))
                    .collect()
            })
    }
}

impl ChatService {
    fn map_to_transfer_object(chat: Chat, user_info: &UserInfo) -> ChatDto {
        let recipient;

        if chat.members.me == user_info.sub {
            recipient = chat.members.you;
        } else if chat.members.you == user_info.sub {
            recipient = chat.members.me;
        } else {
            panic!("You are not a participant of this chat");
        }

        ChatDto::new(
            chat.id.expect("No way chat id is missing!?"),
            recipient,
            &chat.last_message,
        )
    }
}
