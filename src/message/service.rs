use std::sync::Arc;

use log::debug;
use text_splitter::{Characters, TextSplitter};

use crate::chat::service::ChatValidator;
use crate::event::service::EventService;
use crate::{chat, event, user};

use super::model::Message;
use super::repository::MessageRepository;
use super::Id;

const MAX_MESSAGE_LENGTH: usize = 1000;

#[derive(Clone)]
pub struct MessageService {
    repository: Arc<MessageRepository>,
    chat_validator: Arc<ChatValidator>,
    event_service: Arc<EventService>,
    splitter: Arc<TextSplitter<Characters>>,
}

impl MessageService {
    pub fn new(
        repository: MessageRepository,
        chat_validator: ChatValidator,
        event_service: EventService,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            chat_validator: Arc::new(chat_validator),
            event_service: Arc::new(event_service),
            splitter: Arc::new(TextSplitter::new(MAX_MESSAGE_LENGTH)),
        }
    }
}

impl MessageService {
    pub async fn create(&self, msg: &Message) -> super::Result<Vec<Message>> {
        if msg.text.is_empty() {
            return Err(super::Error::EmptyText);
        }

        let messages = match msg.text.len() {
            text_length if text_length <= MAX_MESSAGE_LENGTH => {
                self.repository.insert(msg).await?;
                vec![msg.clone()]
            }
            _ => {
                let messages = split_message(&self.splitter, msg);
                self.repository.insert_many(&messages).await?;
                messages
            }
        };

        // TODO: publish_all
        for msg in &messages {
            self.event_service
                .publish(
                    &event::Subject::Messages(msg.recipient.clone(), msg.chat_id.clone()),
                    event::Message::New(msg.clone()),
                )
                .await?;
        }

        Ok(messages)
    }

    pub async fn find_most_recent(&self, chat_id: &chat::Id) -> super::Result<Option<Message>> {
        self.repository.find_most_recent(chat_id).await
    }

    // pub async fn update(&self, id: &Id, text: &str) -> super::Result<()> {
    //     self.repository.update(id, text).await
    // }

    pub async fn delete(&self, owner: &user::Sub, id: &Id) -> super::Result<Option<Message>> {
        let msg = self.repository.find_by_id(id).await?;
        let chat_id = &msg.chat_id;
        self.chat_validator
            .check_member(chat_id, owner)
            .await
            .map_err(|_| super::Error::NotOwner)?;

        let deleted_count = self.repository.delete(id).await?;

        self.event_service
            .publish(
                &event::Subject::Messages(msg.recipient.clone(), chat_id.clone()),
                event::Message::Deleted(id.clone()),
            )
            .await?;

        if deleted_count > 0 {
            return Ok(Some(msg));
        }

        Ok(None)
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
    ) -> super::Result<(Vec<Message>, usize)> {
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

        let seen_qty = self.mark_as_seen(logged_sub, &messages).await?;

        Ok((messages, seen_qty))
    }

    pub async fn mark_as_seen(
        &self,
        logged_sub: &user::Sub,
        messages: &[Message],
    ) -> super::Result<usize> {
        if messages.is_empty() {
            debug!("attempting to mark as seen but messages list is empty");
            return Ok(0);
        }

        let owner = messages
            .iter()
            .find(|msg| msg.recipient.eq(logged_sub))
            .map(|msg| msg.owner.clone());

        if owner.is_none() {
            debug!("all messages belong to logged user, skipping mark as seen");
            return Ok(0);
        }

        let messages = messages
            .iter()
            .filter(|msg| msg.recipient.eq(logged_sub))
            .filter(|msg| !msg.seen)
            .collect::<Vec<_>>();

        if messages.is_empty() {
            debug!("all messages are already seen, skipping mark as seen");
            return Ok(0);
        }

        let ids = messages
            .iter()
            .map(|msg| msg._id.clone())
            .collect::<Vec<_>>();

        self.repository.mark_as_seen(&ids).await?;

        let owner = owner.expect("no owner present");
        let seen_qty = messages.len();
        for msg in messages {
            self.event_service
                .publish(
                    &event::Subject::Messages(owner.clone(), msg.chat_id.clone()),
                    event::Message::Seen(msg.clone()),
                )
                .await?;
        }

        Ok(seen_qty)
    }
}

fn split_message(splitter: &TextSplitter<Characters>, msg: &Message) -> Vec<Message> {
    let chunks = splitter.chunks(&msg.text);

    chunks
        .map(|text| msg.with_random_id().with_text(text))
        .collect::<Vec<Message>>()
}
