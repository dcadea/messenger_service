use std::sync::Arc;

use futures::future::try_join_all;
use futures::TryFutureExt;
use log::warn;

use super::model::{Chat, ChatDto};
use super::repository::ChatRepository;
use super::Id;
use crate::event::model::{Notification, Queue};
use crate::event::service::EventService;
use crate::integration::{self, cache};
use crate::message::model::{LastMessage, Message};
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{chat, user};

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<ChatRepository>,
    user_service: Arc<UserService>,
    event_service: Arc<EventService>,
    redis: integration::cache::Redis,
}

impl ChatService {
    pub fn new(
        repository: ChatRepository,
        user_service: UserService,
        event_service: EventService,
        redis: integration::cache::Redis,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            user_service: Arc::new(user_service),
            event_service: Arc::new(event_service),
            redis,
        }
    }
}

impl ChatService {
    pub async fn update_last_message(
        &self,
        id: &Id,
        msg: Option<LastMessage>,
    ) -> super::Result<()> {
        self.repository.update_last_message(id, msg).await
    }

    pub async fn is_last_message(&self, message: &Message) -> super::Result<bool> {
        let chat = self.repository.find_by_id(&message.chat_id).await?;

        if let Some(last_message) = chat.last_message {
            return Ok(last_message.id == message._id);
        }

        Ok(false)
    }

    pub async fn find_by_id(&self, id: &Id, user_info: &UserInfo) -> super::Result<ChatDto> {
        match self.repository.find_by_id_and_sub(id, &user_info.sub).await {
            Ok(chat) => {
                let chat_dto = self.chat_to_dto(chat, user_info).await?;
                Ok(chat_dto)
            }
            Err(chat::Error::NotFound(_)) => Err(chat::Error::NotMember),
            Err(err) => Err(err),
        }
    }

    pub async fn find_all(&self, user_info: &UserInfo) -> super::Result<Vec<ChatDto>> {
        let chats = self.repository.find_by_sub(&user_info.sub).await?;

        let chat_dtos = try_join_all(
            chats
                .into_iter()
                .map(|chat| async { self.chat_to_dto(chat, user_info).await }),
        )
        .await?;

        Ok(chat_dtos)
    }

    pub async fn create(
        &self,
        logged_user: &UserInfo,
        recipient: &user::Sub,
    ) -> super::Result<ChatDto> {
        let members = [logged_user.sub.clone(), recipient.clone()];
        if self.repository.exists(&members).await? {
            return Err(chat::Error::AlreadyExists);
        }

        self.user_service.create_friendship(&members).await?;

        let chat = Chat::private(members);
        _ = self.repository.create(chat.clone()).await?;

        let chat_dto = self.chat_to_dto(chat, logged_user).await?;

        if let Err(e) = self
            .event_service
            .publish(
                Queue::Notifications(recipient.clone()),
                Notification::NewFriend {
                    chat_dto: chat_dto.clone(),
                },
            )
            .await
        {
            warn!("Failed to publish new friend notification: {:?}", e);
        }

        Ok(chat_dto)
    }
}

// validations
impl ChatService {
    pub async fn check_member(&self, chat_id: &Id, sub: &user::Sub) -> super::Result<()> {
        let members = self.find_members(chat_id).await?;
        let belongs_to_chat = members.contains(sub);

        if !belongs_to_chat {
            return Err(chat::Error::NotMember);
        }

        Ok(())
    }
}

// cache operations
impl ChatService {
    pub async fn find_members(&self, chat_id: &Id) -> super::Result<Vec<user::Sub>> {
        let cache_key = cache::Key::Chat(chat_id.to_owned());
        let members: Option<Vec<user::Sub>> = self.redis.smembers(cache_key.clone()).await?;

        match members {
            Some(m) if !m.is_empty() => Ok(m),
            _ => {
                let chat = self.repository.find_by_id(chat_id).await?;
                let members = chat.members;

                let _: () = self
                    .redis
                    .sadd(cache_key.clone(), &members.clone())
                    .and_then(|_: ()| self.redis.expire(cache_key))
                    .await?;

                Ok(members)
            }
        }
    }
}

impl ChatService {
    // FIXME: this is for private chat only
    async fn chat_to_dto(&self, chat: Chat, user_info: &UserInfo) -> super::Result<ChatDto> {
        let members = chat.members.to_owned();

        let recipient = members
            .iter()
            .find(|&m| m != &user_info.sub) // someone who is not a logged user :)
            .ok_or(chat::Error::NotMember)?;

        let recipient_info = self
            .user_service
            .find_user_info(recipient)
            .await
            .expect("recipient info should be present");

        let chat_dto = ChatDto::new(chat, recipient.to_owned(), recipient_info.name);

        Ok(chat_dto)
    }
}
