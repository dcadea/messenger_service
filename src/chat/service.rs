use std::collections::HashSet;
use std::sync::Arc;

use futures::future::try_join_all;
use log::error;

use super::model::{Chat, ChatDto};
use super::repository::ChatRepository;
use crate::event::service::EventService;
use crate::integration::cache;
use crate::message::repository::MessageRepository;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{chat, event, user};

#[derive(Clone)]
pub struct ChatService {
    repo: Arc<ChatRepository>,
    message_repo: Arc<MessageRepository>,
    user_service: Arc<UserService>,
    event_service: Arc<EventService>,
    redis: cache::Redis,
}

impl ChatService {
    pub fn new(
        repo: ChatRepository,
        message_repo: MessageRepository,
        user_service: UserService,
        event_service: EventService,
        redis: cache::Redis,
    ) -> Self {
        Self {
            repo: Arc::new(repo),
            message_repo: Arc::new(message_repo),
            user_service: Arc::new(user_service),
            event_service: Arc::new(event_service),
            redis,
        }
    }
}

impl ChatService {
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
