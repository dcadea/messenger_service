use std::sync::Arc;

use futures::future::try_join_all;
use futures::TryFutureExt;
use redis::AsyncCommands;

use super::model::{Chat, ChatDto};
use super::repository::ChatRepository;
use super::Id;
use crate::integration::cache;
use crate::message::model::Message;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{chat, user};

const CHAT_TTL: i64 = 3600;

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<ChatRepository>,
    user_service: Arc<UserService>,
    redis_con: redis::aio::ConnectionManager,
}

impl ChatService {
    pub fn new(
        repository: ChatRepository,
        user_service: UserService,
        redis_con: redis::aio::ConnectionManager,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            user_service: Arc::new(user_service),
            redis_con,
        }
    }
}

impl ChatService {
    pub async fn update_last_message(&self, message: &Message) -> super::Result<()> {
        let chat_id = self
            .repository
            .find_id_by_members([message.owner.to_owned(), message.recipient.to_owned()])
            .await?;

        self.repository
            .update_last_message(&chat_id, &message.text)
            .await
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

    pub async fn check_members(&self, chat_id: &Id, members: [&user::Sub; 2]) -> super::Result<()> {
        let cached_members = self.find_members(chat_id).await?;
        let belongs_to_chat =
            cached_members.contains(members[0]) && cached_members.contains(members[1]);

        if !belongs_to_chat {
            return Err(chat::Error::NotMember);
        }

        Ok(())
    }
}

// cache operations
impl ChatService {
    pub async fn find_members(&self, chat_id: &Id) -> super::Result<[user::Sub; 2]> {
        let mut con = self.redis_con.clone();

        let cache_key = cache::Key::Chat(chat_id.to_owned());
        let members: Option<Vec<user::Sub>> = con.smembers(cache_key.clone()).await?;

        if members.as_ref().is_some_and(|m| m.len() == 2) {
            let members = members.unwrap();
            return Ok([members[0].clone(), members[1].clone()]);
        }

        let chat = self.repository.find_by_id(chat_id).await?;
        let members = chat.members;

        let _: () = con
            .clone()
            .sadd(&cache_key, &members.clone())
            .and_then(|_: ()| con.expire(&cache_key, CHAT_TTL))
            .await?;

        Ok(members)
    }
}

impl ChatService {
    async fn chat_to_dto(&self, chat: Chat, user_info: &UserInfo) -> super::Result<ChatDto> {
        let members = chat.members.to_owned();

        let recipient = members
            .iter()
            .find(|&m| m != &user_info.sub) // someone who is not a logged user :)
            .ok_or(chat::Error::NotMember)?;

        let recipient_info = self.user_service.find_user_info(recipient).await?;

        let chat_dto = ChatDto::new(chat, recipient.to_owned(), recipient_info.name);

        Ok(chat_dto)
    }
}
