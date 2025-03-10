use std::sync::Arc;

use log::debug;
use text_splitter::{Characters, TextSplitter};

use crate::chat::service::ChatValidator;
use crate::event::service::EventService;
use crate::{chat, event, user};

use super::Id;
use super::model::Message;
use super::repository::MessageRepository;

const MAX_MESSAGE_LENGTH: usize = 1000;

#[derive(Clone)]
pub struct MessageService {
    repo: Arc<MessageRepository>,
    chat_validator: Arc<ChatValidator>,
    event_service: Arc<EventService>,
    splitter: Arc<TextSplitter<Characters>>,
}

impl MessageService {
    pub fn new(
        repo: MessageRepository,
        chat_validator: ChatValidator,
        event_service: EventService,
    ) -> Self {
        Self {
            repo: Arc::new(repo),
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

        let msgs = match msg.text.len() {
            text_length if text_length <= MAX_MESSAGE_LENGTH => {
                self.repo.insert(msg).await?;
                vec![msg.clone()]
            }
            _ => {
                let msgs = split_message(&self.splitter, msg);
                self.repo.insert_many(&msgs).await?;
                msgs
            }
        };

        let msg_evts: Vec<event::Message> = msgs
            .iter()
            .map(|m| event::Message::New(m.clone()))
            .collect();

        self.event_service
            .publish_all(
                &event::Subject::Messages(&msg.recipient, &msg.chat_id),
                &msg_evts,
            )
            .await;

        Ok(msgs)
    }

    pub async fn find_by_id(&self, id: &Id) -> super::Result<Message> {
        self.repo.find_by_id(id).await
    }

    pub async fn find_most_recent(&self, chat_id: &chat::Id) -> super::Result<Option<Message>> {
        self.repo.find_most_recent(chat_id).await
    }

    pub async fn update(&self, owner: &user::Sub, id: &Id, text: &str) -> super::Result<Message> {
        let msg = self.repo.find_by_id(id).await?;

        if msg.owner.ne(owner) {
            return Err(super::Error::NotOwner);
        }

        self.repo.update(id, text).await?;

        let msg = msg.with_text(text);
        self.event_service
            .publish(
                &event::Subject::Messages(&msg.recipient, &msg.chat_id),
                &event::Message::Updated(msg.clone()),
            )
            .await;

        Ok(msg)
    }

    pub async fn delete(&self, owner: &user::Sub, id: &Id) -> super::Result<Option<Message>> {
        let msg = self.repo.find_by_id(id).await?;
        let chat_id = &msg.chat_id;
        self.chat_validator
            .check_member(chat_id, owner)
            .await
            .map_err(|_| super::Error::NotOwner)?;

        let deleted_count = self.repo.delete(id).await?;

        self.event_service
            .publish(
                &event::Subject::Messages(&msg.recipient, chat_id),
                &event::Message::Deleted(id.clone()),
            )
            .await;

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
        let msgs = match (limit, end_time) {
            (None, None) => self.repo.find_by_chat_id(chat_id).await,
            (Some(limit), None) => self.repo.find_by_chat_id_limited(chat_id, limit).await,
            (None, Some(end_time)) => self.repo.find_by_chat_id_before(chat_id, end_time).await,
            (Some(limit), Some(end_time)) => {
                self.repo
                    .find_by_chat_id_limited_before(chat_id, limit, end_time)
                    .await
            }
        }?;

        let seen_qty = self.mark_as_seen(logged_sub, &msgs).await?;

        Ok((msgs, seen_qty))
    }

    pub async fn mark_as_seen(
        &self,
        logged_sub: &user::Sub,
        msgs: &[Message],
    ) -> super::Result<usize> {
        if msgs.is_empty() {
            debug!("attempting to mark as seen but messages list is empty");
            return Ok(0);
        }

        let owner = msgs
            .iter()
            .find(|msg| msg.recipient.eq(logged_sub))
            .map(|msg| msg.owner.clone());

        if owner.is_none() {
            debug!("all messages belong to logged user, skipping mark as seen");
            return Ok(0);
        }

        let msgs = msgs
            .iter()
            .filter(|msg| msg.recipient.eq(logged_sub))
            .filter(|msg| !msg.seen)
            .collect::<Vec<_>>();

        if msgs.is_empty() {
            debug!("all messages are already seen, skipping mark as seen");
            return Ok(0);
        }

        let ids = msgs.iter().map(|msg| msg._id.clone()).collect::<Vec<_>>();

        self.repo.mark_as_seen(&ids).await?;

        let owner = owner.expect("no owner present");
        let chat_id = msgs
            .first()
            .map(|m| m.chat_id.clone())
            .expect("chat_id must be present");
        let msg_evts: Vec<event::Message> = msgs
            .iter()
            .map(|m| event::Message::Seen((*m).clone()))
            .collect();

        self.event_service
            .publish_all(&event::Subject::Messages(&owner, &chat_id), &msg_evts)
            .await;

        let seen_qty = msgs.len();
        Ok(seen_qty)
    }
}

fn split_message(splitter: &TextSplitter<Characters>, msg: &Message) -> Vec<Message> {
    let chunks = splitter.chunks(&msg.text);

    chunks
        .map(|text| msg.with_random_id().with_text(text))
        .collect::<Vec<Message>>()
}
