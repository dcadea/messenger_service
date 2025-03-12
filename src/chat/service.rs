use std::collections::HashSet;
use std::sync::Arc;

use futures::future::try_join_all;
use log::error;

use super::model::{Chat, ChatDto};
use super::repository::ChatRepository;
use crate::event::service::EventService;
use crate::integration::cache;
use crate::message::model::{LastMessage, Message};
use crate::message::repository::MessageRepository;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{chat, event, user};

#[derive(Clone)]
pub struct ChatService {
    repo: Arc<ChatRepository>,
    validator: Arc<ChatValidator>,
    message_repo: Arc<MessageRepository>,
    user_service: Arc<UserService>,
    event_service: Arc<EventService>,
}

impl ChatService {
    pub fn new(
        repo: ChatRepository,
        validator: ChatValidator,
        message_repo: MessageRepository,
        user_service: UserService,
        event_service: EventService,
    ) -> Self {
        Self {
            repo: Arc::new(repo),
            validator: Arc::new(validator),
            message_repo: Arc::new(message_repo),
            user_service: Arc::new(user_service),
            event_service: Arc::new(event_service),
        }
    }
}

impl ChatService {
    pub async fn find_by_id(&self, id: &chat::Id, user_info: &UserInfo) -> super::Result<ChatDto> {
        let sub = &user_info.sub;
        match self.repo.find_by_id_and_sub(id, sub).await {
            Ok(chat) => {
                let chat_dto = self.chat_to_dto(chat, sub).await?;
                Ok(chat_dto)
            }
            Err(chat::Error::NotFound(_)) => Err(chat::Error::NotMember),
            Err(err) => Err(err),
        }
    }

    pub async fn find_all(&self, user_info: &UserInfo) -> super::Result<Vec<ChatDto>> {
        let sub = &user_info.sub;
        let chats = self.repo.find_by_sub(sub).await?;

        let chat_dtos = try_join_all(
            chats
                .into_iter()
                .map(|chat| async { self.chat_to_dto(chat, sub).await }),
        )
        .await?;

        Ok(chat_dtos)
    }

    pub async fn create(
        &self,
        logged_user: &UserInfo,
        kind: chat::Kind,
        recipient: &user::Sub,
    ) -> super::Result<ChatDto> {
        assert_ne!(&logged_user.sub, recipient);

        let members = [logged_user.sub.clone(), recipient.clone()];
        if self.repo.exists(&members).await? {
            return Err(chat::Error::AlreadyExists);
        }

        if let Err(e) = self.user_service.create_friendship(&members).await {
            error!("could not create friendship: {e:?}");
            return Err(chat::Error::NotCreated);
        }

        let chat = Chat::new(kind, logged_user.sub.clone(), HashSet::from(members));
        _ = self.repo.create(chat.clone()).await?;

        let chat_dto = self.chat_to_dto(chat, recipient).await?;

        self.event_service
            .publish(
                &event::Subject::Notifications(recipient),
                &event::Notification::NewFriend(chat_dto.clone()),
            )
            .await;

        Ok(chat_dto)
    }

    pub async fn delete(&self, id: &chat::Id, logged_user: &UserInfo) -> super::Result<()> {
        self.validator.check_member(id, &logged_user.sub).await?;

        let chat = self.find_by_id(id, logged_user).await?;

        let friends = [chat.sender, chat.recipient];
        if let Err(e) = self.user_service.delete_friendship(&friends).await {
            error!("could not delete friendship: {e:?}");
            return Err(chat::Error::NotDeleted);
        }

        self.repo.delete(id).await?;
        if let Err(e) = self.message_repo.delete_by_chat_id(id).await {
            error!("failed to delete chat: {e:?}");
            return Err(chat::Error::NotDeleted);
            // TODO: tx rollback?
        }

        Ok(())
    }
}

impl ChatService {
    pub async fn update_last_message(
        &self,
        id: &chat::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()> {
        self.repo.update_last_message(id, msg).await?;

        if let Some(last_msg) = msg {
            // FIXME: extract find_members from validator
            let mut recipients = self.validator.find_members(id).await?;
            recipients.remove(&last_msg.owner);

            for r in recipients {
                self.event_service
                    .publish(
                        &event::Subject::Notifications(&r),
                        &event::Notification::NewMessage {
                            chat_id: id.clone(),
                            last_message: last_msg.clone(),
                        },
                    )
                    .await;
            }
        }
        Ok(())
    }

    pub async fn is_last_message(&self, message: &Message) -> super::Result<bool> {
        let chat = self.repo.find_by_id(&message.chat_id).await?;

        if let Some(last_message) = chat.last_message {
            return Ok(last_message.id == message._id);
        }

        Ok(false)
    }

    pub async fn mark_as_seen(&self, id: &chat::Id) -> super::Result<()> {
        self.repo.mark_as_seen(id).await
    }
}

impl ChatService {
    // FIXME: this is for private chat only
    async fn chat_to_dto(&self, chat: Chat, sub: &user::Sub) -> super::Result<ChatDto> {
        let members = chat.members.to_owned();

        let sender = members
            .iter()
            .find(|&m| m == sub)
            .ok_or(chat::Error::NotMember)?;

        let recipient = members
            .iter()
            .find(|&m| m != sub)
            .ok_or(chat::Error::NotMember)?;

        let recipient_info = self
            .user_service
            .find_user_info(recipient)
            .await
            .expect("recipient info should be present");

        let chat_dto = ChatDto::new(
            chat,
            sender.to_owned(),
            recipient.to_owned(),
            recipient_info.picture,
            recipient_info.name,
        );

        Ok(chat_dto)
    }
}

#[derive(Clone)]
pub struct ChatValidator {
    repository: Arc<ChatRepository>,
    redis: cache::Redis,
}

impl ChatValidator {
    pub fn new(repository: ChatRepository, redis: cache::Redis) -> Self {
        Self {
            repository: Arc::new(repository),
            redis,
        }
    }
}

impl ChatValidator {
    pub async fn check_member(&self, chat_id: &chat::Id, sub: &user::Sub) -> super::Result<()> {
        let members = self.find_members(chat_id).await?;
        let belongs_to_chat = members.contains(sub);

        if !belongs_to_chat {
            return Err(chat::Error::NotMember);
        }

        Ok(())
    }

    async fn find_members(&self, chat_id: &chat::Id) -> super::Result<HashSet<user::Sub>> {
        let chat_key = cache::Key::Chat(chat_id.to_owned());
        let members = self
            .redis
            .smembers::<HashSet<user::Sub>>(chat_key.clone())
            .await;

        match members {
            Some(m) if !m.is_empty() => Ok(m),
            _ => {
                let chat = self.repository.find_by_id(chat_id).await?;
                let members = chat.members;

                self.redis.sadd(chat_key.clone(), &members).await;
                self.redis.expire(chat_key).await;

                Ok(members)
            }
        }
    }
}
