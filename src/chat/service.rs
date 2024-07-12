use super::model::{Chat, ChatDto, ChatId, ChatRequest, Members};
use super::repository::ChatRepository;
use super::Result;
use crate::chat::error::ChatError;
use crate::integration::model::CacheKey;
use crate::message::model::Message;
use crate::model::{AppEndpoints, LinkFactory};
use crate::user::model::{UserInfo, UserSub};
use crate::user::service::UserService;
use futures::future::try_join_all;
use futures::TryFutureExt;
use redis::AsyncCommands;
use std::collections::HashSet;
use std::sync::Arc;

const CHAT_TTL: i64 = 3600;

#[derive(Clone)]
pub struct ChatService {
    repository: Arc<ChatRepository>,
    user_service: Arc<UserService>,
    redis_con: redis::aio::ConnectionManager,
    link_factory: Arc<LinkFactory>,
}

impl ChatService {
    pub fn new(
        repository: ChatRepository,
        user_service: UserService,
        redis_con: redis::aio::ConnectionManager,
        app_endpoints: AppEndpoints,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            user_service: Arc::new(user_service),
            redis_con,
            link_factory: Arc::new(LinkFactory::new(app_endpoints.api())),
        }
    }
}

impl ChatService {
    pub async fn create(
        &self,
        chat_request: &ChatRequest,
        user_info: &UserInfo,
    ) -> Result<ChatDto> {
        let members = Members::new(user_info.sub.clone(), chat_request.recipient.clone());

        match self.repository.find_id_by_members(&members).await {
            Ok(_) => Err(ChatError::AlreadyExists(members)),
            Err(ChatError::NotFound(_)) => {
                let chat = self.repository.insert(&Chat::new(members)).await?;
                self.chat_to_dto(chat, user_info).await
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
        match self.repository.find_by_id_and_sub(id, &user_info.sub).await {
            Ok(chat) => Ok(self.chat_to_dto(chat, user_info).await?),
            Err(ChatError::NotFound(_)) => Err(ChatError::NotMember),
            Err(err) => Err(err),
        }
    }

    pub async fn find_all(&self, user_info: &UserInfo) -> Result<Vec<ChatDto>> {
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
    pub async fn check_member(&self, chat_id: ChatId, user_info: &UserInfo) -> Result<()> {
        let members = self.find_members(chat_id).await?;

        if members.contains(&user_info.sub) {
            Ok(())
        } else {
            Err(ChatError::NotMember)
        }
    }

    pub async fn check_members(&self, chat_id: ChatId, members: &Members) -> Result<()> {
        let cached_members = self.find_members(chat_id).await?;
        let belongs_to_chat = cached_members.intersection(&members.to_set()).count() > 0;

        if belongs_to_chat {
            Ok(())
        } else {
            Err(ChatError::NotMember)
        }
    }
}

// cache operations
impl ChatService {
    pub async fn find_members(&self, chat_id: ChatId) -> Result<HashSet<UserSub>> {
        let mut con = self.redis_con.clone();

        let cache_key = CacheKey::Chat(chat_id);
        let members: Option<HashSet<UserSub>> = con.smembers(cache_key.clone()).await?;

        match members {
            Some(members) => Ok(members),
            None => {
                let chat = self.repository.find_by_id(&chat_id).await?;
                let members = chat.members.to_set();

                let _: () = con
                    .clone()
                    .sadd(cache_key.clone(), members.clone())
                    .and_then(|_: ()| con.expire(cache_key.clone(), CHAT_TTL))
                    .await?;
                Ok(members)
            }
        }
    }
}

impl ChatService {
    async fn chat_to_dto(&self, chat: Chat, user_info: &UserInfo) -> Result<ChatDto> {
        let recipient;

        if chat.members.me == user_info.sub {
            recipient = chat.members.clone().you;
        } else if chat.members.you == user_info.sub {
            recipient = chat.members.clone().me;
        } else {
            return Err(ChatError::NotMember);
        }

        let recipient_info = self.user_service.find_user_info(recipient.clone()).await?;

        let chat_dto = ChatDto::from_chat(chat, recipient_info.name);
        let links = vec![
            self.link_factory._self(&format!("chats/{}", chat_dto.id)),
            self.link_factory
                .recipient(&format!("users?sub={recipient}")),
        ];

        Ok(chat_dto.with_links(links))
    }
}
