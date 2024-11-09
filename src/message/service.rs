use std::sync::Arc;

use anyhow::Context;

use crate::chat::service::ChatService;
use crate::event::model::Notification;
use crate::event::service::EventService;
use crate::{chat, user};

use super::model::{Message, MessageDto};
use super::repository::MessageRepository;
use super::Id;

#[derive(Clone)]
pub struct MessageService {
    repository: Arc<MessageRepository>,
    chat_service: Arc<ChatService>,
    event_service: Arc<EventService>,
}

impl MessageService {
    pub fn new(
        repository: MessageRepository,
        chat_service: ChatService,
        event_service: EventService,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            chat_service: Arc::new(chat_service),
            event_service: Arc::new(event_service),
        }
    }
}

impl MessageService {
    pub async fn create(&self, message: &Message) -> super::Result<MessageDto> {
        let dto = self
            .repository
            .insert(message)
            .await
            .map(|id| message.with_id(id))
            .map(MessageDto::from)?;

        self.chat_service
            .update_last_message(message)
            .await
            .with_context(|| "Failed to update last message in chat")?;

        self.event_service
            .publish_noti(
                &dto.recipient.clone().into(),
                &Notification::NewMessage { dto: dto.clone() },
            )
            .await
            .with_context(|| "Failed to publish notification")?;

        Ok(dto)
    }

    // pub async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
    //     self.repository.update(id, text).await
    // }

    pub async fn delete(&self, owner: &user::Sub, id: &Id) -> super::Result<()> {
        let msg = self.repository.find_by_id(id).await?;
        self.chat_service
            .check_member(&msg.chat_id, owner)
            .await
            .map_err(|_| super::Error::NotOwner)?;

        self.repository.delete(id).await?;

        self.event_service
            .publish_noti(
                &msg.recipient.clone().into(),
                &Notification::DeletedMessage { id: id.to_owned() },
            )
            .await
            .with_context(|| "Failed to publish notification")?;

        Ok(())
    }

    // pub async fn mark_as_seen(&self, id: &Id) -> super::Result<()> {
    //     self.repository.mark_as_seen(id).await
    // }
}

impl MessageService {
    pub async fn find_by_id(&self, id: &Id) -> super::Result<MessageDto> {
        self.repository.find_by_id(id).await.map(MessageDto::from)
    }

    pub async fn find_by_chat_id_and_params(
        &self,
        chat_id: &chat::Id,
        limit: Option<usize>,
        end_time: Option<i64>,
    ) -> super::Result<Vec<MessageDto>> {
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
            .map(|msg| MessageDto::from(msg.clone()))
            .collect::<Vec<_>>();

        Ok(result)
    }
}
