use std::sync::Arc;

use anyhow::Context;
use log::debug;
use text_splitter::{Characters, TextSplitter};

use crate::chat::service::ChatService;
use crate::event::model::{Notification, Queue};
use crate::event::service::EventService;
use crate::{chat, user};

use super::model::{Message, MessageDto};
use super::repository::MessageRepository;
use super::Id;

const MAX_MESSAGE_LENGTH: usize = 1000;

#[derive(Clone)]
pub struct MessageService {
    repository: Arc<MessageRepository>,
    chat_service: Arc<ChatService>,
    event_service: Arc<EventService>,
    splitter: Arc<TextSplitter<Characters>>,
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
            splitter: Arc::new(TextSplitter::new(MAX_MESSAGE_LENGTH)),
        }
    }
}

impl MessageService {
    pub async fn create(&self, message: &Message) -> super::Result<Vec<MessageDto>> {
        if message.text.is_empty() {
            return Err(super::Error::EmptyText);
        }

        // TODO: len is not the same as count of characters
        if message.text.len() <= MAX_MESSAGE_LENGTH {
            self.create_one(message).await.map(|msg| vec![msg])
        } else {
            self.create_many(message).await
        }
    }

    async fn create_one(&self, message: &Message) -> super::Result<MessageDto> {
        let msg = self
            .repository
            .insert(message)
            .await
            .map(|id| message.with_id(id))?;

        self.chat_service
            .update_last_message(message)
            .await
            .with_context(|| "Failed to update last message in chat")?;

        let dto = MessageDto::from(&msg);
        self.event_service
            .publish(
                Queue::Notifications(msg.recipient.clone()),
                Notification::NewMessage { msg },
            )
            .await
            .with_context(|| "Failed to publish notification")?;

        Ok(dto)
    }

    async fn create_many(&self, message: &Message) -> super::Result<Vec<MessageDto>> {
        let chunks = self.splitter.chunks(&message.text);

        let messages = chunks
            .map(|text| message.with_text(text))
            .collect::<Vec<Message>>();

        let mut with_ids = Vec::with_capacity(messages.len());
        for msg in &messages {
            // TODO: maybe use insert_many (find a way to batch notifications through ws)
            self.create_one(msg).await.map(|m| with_ids.push(m))?
        }

        Ok(with_ids)
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
            .publish(
                Queue::Notifications(msg.recipient.clone()),
                Notification::DeletedMessage { id: id.to_owned() },
            )
            .await
            .with_context(|| "Failed to publish notification")?;

        Ok(())
    }
}

impl MessageService {
    // This method is designed to be callen when recipient requests messages related to selected chat.
    // It also marks all messages as seen where logged user is recipient.
    // Due to this side effect consider using other methods for read-only messages retrieval.
    pub async fn find_by_chat_id_and_params(
        &self,
        logged_sub: &user::Sub,
        chat_id: &chat::Id,
        limit: Option<usize>,
        end_time: Option<i64>,
    ) -> super::Result<Vec<MessageDto>> {
        let messages = match (limit, end_time) {
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

        self.mark_as_seen(logged_sub, &messages).await?;

        let dtos = messages.iter().map(MessageDto::from).collect::<Vec<_>>();

        Ok(dtos)
    }

    pub async fn mark_as_seen(
        &self,
        logged_sub: &user::Sub,
        messages: &[Message],
    ) -> super::Result<()> {
        if messages.is_empty() {
            debug!("attempting to mark as seen but messages list is empty");
            return Ok(());
        }

        let owner = messages
            .iter()
            .find(|msg| msg.recipient.eq(logged_sub))
            .map(|msg| msg.owner.clone());

        if owner.is_none() {
            debug!("all messages belong to logged user, skipping mark as seen");
            return Ok(());
        }

        let ids = messages
            .iter()
            .filter(|msg| msg.recipient.eq(logged_sub))
            .filter(|msg| !msg.seen)
            .filter_map(|msg| msg.id.clone())
            .collect::<Vec<_>>();

        if ids.is_empty() {
            debug!("all messages are already seen, skipping mark as seen");
            return Ok(());
        }

        self.repository.mark_as_seen(&ids).await?;

        let owner = owner.expect("no owner present");
        for id in &ids {
            self.event_service
                .publish(
                    Queue::Notifications(owner.clone()),
                    Notification::SeenMessage { id: id.clone() },
                )
                .await
                .with_context(|| "Failed to publish notification")?;
        }

        Ok(())
    }
}
