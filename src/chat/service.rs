use crate::chat::error::ChatError;
use std::sync::Arc;

use crate::message::model::Message;
use crate::user::model::UserInfo;

use super::model::{Chat, ChatDto, ChatId, ChatRequest, Members};
use super::repository::ChatRepository;
use super::Result;

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
    pub async fn create(&self, chat_request: &ChatRequest, user_info: &UserInfo) -> Result<ChatId> {
        let members = Members::new(user_info.sub.clone(), chat_request.recipient.clone());

        match self.repository.find_id_by_members(&members).await {
            Ok(_) => Err(ChatError::AlreadyExists(members)),
            Err(ChatError::NotFound(_)) => {
                let chat = Chat::new(members);
                self.repository.insert(&chat).await
            }
            Err(err) => Err(err),
        }
    }

    pub async fn update_last_message(&self, message: &Message) -> Result<()> {
        let members = Members::new(message.owner.clone(), message.recipient.clone());

        let chat_id = self.repository.find_id_by_members(&members).await?;
        self.repository
            .update_last_message(&chat_id, &message.text)
            .await
    }

    pub async fn find_by_id(&self, id: ChatId, user_info: &UserInfo) -> Result<ChatDto> {
        self.repository
            .find_by_id(id)
            .await
            .map(|chat| Self::chat_to_dto(chat, &user_info))
    }

    pub async fn find_all(&self, user_info: &UserInfo) -> Result<Vec<ChatDto>> {
        self.repository
            .find_by_sub(&user_info.sub)
            .await
            .map(|chats| {
                chats
                    .into_iter()
                    .map(|chat| Self::chat_to_dto(chat, &user_info))
                    .collect()
            })
    }
}

impl ChatService {
    fn chat_to_dto(chat: Chat, user_info: &UserInfo) -> ChatDto {
        let recipient;

        if chat.members.me == user_info.sub {
            recipient = chat.members.you;
        } else if chat.members.you == user_info.sub {
            recipient = chat.members.me;
        } else {
            // TODO: return result with proper error
            panic!("You are not a participant of this chat");
        }

        ChatDto::new(
            chat.id.expect("No way chat id is missing!?"),
            recipient,
            chat.last_message,
        )
    }
}
