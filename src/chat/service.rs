use std::sync::Arc;

use futures::future::try_join_all;
use futures::TryFutureExt;
use redis::AsyncCommands;

use crate::chat::error::ChatError;
use crate::integration::model::CacheKey;
use crate::message::model::Message;
use crate::model::{AppEndpoints, LinkFactory};
use crate::user::model::{UserInfo, UserSub};
use crate::user::service::UserService;

use super::model::{Chat, ChatDto, ChatId, ChatRequest};
use super::repository::ChatRepository;
use super::Result;

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
    pub async fn create(&self, req: &ChatRequest, user_info: &UserInfo) -> Result<ChatDto> {
        let owner = user_info.clone().sub;
        let recipient = req.clone().recipient;

        match self
            .repository
            .find_id_by_members([&owner, &recipient])
            .await
        {
            Ok(_) => Err(ChatError::AlreadyExists([owner, recipient])),
            Err(ChatError::NotFound(_)) => {
                let chat = self
                    .repository
                    .insert(&Chat::new([owner.clone(), recipient.clone()]))
                    .await?;

                self.user_service.add_friend(owner, recipient).await?;

                self.chat_to_dto(chat, user_info).await
            }
            Err(err) => Err(err),
        }
    }

    pub async fn update_last_message(&self, message: &Message) -> Result<()> {
        let chat_id = self
            .repository
            .find_id_by_members([&message.owner, &message.recipient])
            .await?;

        self.repository
            .update_last_message(&chat_id, &message.text)
            .await
    }

    pub async fn find_by_id(&self, id: ChatId, user_info: &UserInfo) -> Result<ChatDto> {
        match self.repository.find_by_id_and_sub(id, &user_info.sub).await {
            Ok(chat) => {
                let chat_dto = self.chat_to_dto(chat, user_info).await?;
                // TODO: add friends
                Ok(chat_dto)
            }
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

        // TODO: add friends

        Ok(chat_dtos)
    }
}

// validations
impl ChatService {
    pub async fn check_member(&self, chat_id: ChatId, sub: &UserSub) -> Result<()> {
        let members = self.find_members(chat_id).await?;
        let belongs_to_chat = members.contains(sub);

        if !belongs_to_chat {
            return Err(ChatError::NotMember);
        }

        Ok(())
    }

    pub async fn check_members(&self, chat_id: ChatId, members: [UserSub; 2]) -> Result<()> {
        let cached_members = self.find_members(chat_id).await?;
        let belongs_to_chat =
            cached_members.contains(&members[0]) && cached_members.contains(&members[1]);

        if !belongs_to_chat {
            return Err(ChatError::NotMember);
        }

        Ok(())
    }
}

// cache operations
impl ChatService {
    pub async fn find_members(&self, chat_id: ChatId) -> Result<[UserSub; 2]> {
        let mut con = self.redis_con.clone();

        let cache_key = CacheKey::Chat(chat_id);
        let members: Option<Vec<UserSub>> = con.smembers(cache_key.clone()).await?;

        if members.clone().is_some_and(|m| m.len() == 2) {
            let members = members.unwrap();
            return Ok([members[0].clone(), members[1].clone()]);
        }

        let chat = self.repository.find_by_id(&chat_id).await?;
        let members = chat.members;

        let _: () = con
            .clone()
            .sadd(cache_key.clone(), &members.clone())
            .and_then(|_: ()| con.expire(cache_key.clone(), CHAT_TTL))
            .await?;

        Ok(members)
    }
}

impl ChatService {
    async fn chat_to_dto(&self, chat: Chat, user_info: &UserInfo) -> Result<ChatDto> {
        let members = chat.members.clone();

        let recipient = members
            .iter()
            .find(|&m| m != &user_info.sub) // someone who is not a logged user :)
            .ok_or(ChatError::NotMember)?;

        let recipient_info = self.user_service.find_user_info(recipient.clone()).await?;

        let chat_dto = ChatDto::new(chat, recipient.clone(), recipient_info.name);

        let links = vec![
            self.link_factory._self(&format!("chats/{}", chat_dto.id)),
            self.link_factory
                .recipient(&format!("users?sub={recipient}")),
        ];

        Ok(chat_dto.with_links(links))
    }
}
