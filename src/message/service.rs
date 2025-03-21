use std::sync::Arc;

use log::{debug, error};
use text_splitter::{Characters, TextSplitter};

use crate::event::service::EventService;
use crate::talk::service::{TalkService, TalkValidator};
use crate::{event, message, talk, user};

use super::model::Message;
use super::repository::MessageRepository;

const MAX_MESSAGE_LENGTH: usize = 1000;

#[derive(Clone)]
pub struct MessageService {
    repo: Arc<MessageRepository>,
    talk_service: Arc<TalkService>,
    talk_validator: Arc<TalkValidator>,
    event_service: Arc<EventService>,
    splitter: Arc<TextSplitter<Characters>>,
}

impl MessageService {
    pub fn new(
        repo: MessageRepository,
        talk_service: TalkService,
        talk_validator: TalkValidator,
        event_service: EventService,
    ) -> Self {
        Self {
            repo: Arc::new(repo),
            talk_service: Arc::new(talk_service),
            talk_validator: Arc::new(talk_validator),
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

        self.notify_new(&msg.talk_id, &msg.owner, &msgs).await;

        Ok(msgs)
    }

    pub async fn find_by_id(&self, id: &message::Id) -> super::Result<Message> {
        self.repo.find_by_id(id).await
    }

    pub async fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>> {
        self.repo.find_most_recent(talk_id).await
    }

    pub async fn update(
        &self,
        logged_sub: &user::Sub,
        id: &message::Id,
        text: &str,
    ) -> super::Result<Message> {
        let msg = self.repo.find_by_id(id).await?;

        if msg.owner.ne(logged_sub) {
            return Err(super::Error::NotOwner);
        }

        self.repo.update(id, text).await?;
        let msg = msg.with_text(text);
        self.notify_updated(&msg).await;

        Ok(msg)
    }

    pub async fn delete(
        &self,
        logged_sub: &user::Sub,
        id: &message::Id,
    ) -> super::Result<Option<Message>> {
        let msg = self.repo.find_by_id(id).await?;
        let talk_id = &msg.talk_id;
        self.talk_validator
            .check_member(talk_id, logged_sub)
            .await
            .map_err(|_| super::Error::NotOwner)?;

        if msg.owner.ne(logged_sub) {
            return Err(super::Error::NotOwner);
        }

        let deleted_count = self.repo.delete(id).await?;

        if deleted_count > 0 {
            self.notify_deleted(&msg).await;
            return Ok(Some(msg));
        }

        Ok(None)
    }
}

impl MessageService {
    // This method is designed to be callen when recipient requests messages related to selected talk.
    // It also marks all messages as seen where logged user is recipient.
    // Due to this side effect consider using other methods for read-only messages retrieval.
    pub async fn find_by_talk_id_and_params(
        &self,
        logged_sub: &user::Sub,
        talk_id: &talk::Id,
        limit: Option<usize>,
        end_time: Option<i64>,
    ) -> super::Result<(Vec<Message>, usize)> {
        let msgs = match (limit, end_time) {
            (None, None) => self.repo.find_by_talk_id(talk_id).await,
            (Some(limit), None) => self.repo.find_by_talk_id_limited(talk_id, limit).await,
            (None, Some(end_time)) => self.repo.find_by_talk_id_before(talk_id, end_time).await,
            (Some(limit), Some(end_time)) => {
                self.repo
                    .find_by_talk_id_limited_before(talk_id, limit, end_time)
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

        let anothers_messages = msgs
            .iter()
            .filter(|msg| msg.owner.ne(logged_sub))
            .collect::<Vec<_>>();

        if anothers_messages.is_empty() {
            debug!("all messages belong to logged user, skipping mark as seen");
            return Ok(0);
        }

        let unseen_msgs = anothers_messages
            .into_iter()
            .filter(|msg| !msg.seen)
            .collect::<Vec<_>>();

        if unseen_msgs.is_empty() {
            debug!("all messages are already seen, skipping mark as seen");
            return Ok(0);
        }

        let unseen_ids = unseen_msgs
            .iter()
            .map(|msg| msg._id.clone())
            .collect::<Vec<_>>();

        self.repo.mark_as_seen(&unseen_ids).await?;

        let msg_evts: Vec<event::Message> = unseen_msgs
            .iter()
            .map(|m| event::Message::Seen((*m).clone()))
            .collect();

        for msg in unseen_msgs {
            self.event_service
                .publish_all(
                    &event::Subject::Messages(&msg.owner, &msg.talk_id),
                    &msg_evts,
                )
                .await;
        }

        let seen_qty = msg_evts.len();
        Ok(seen_qty)
    }

    pub async fn is_last_message(&self, msg: &Message) -> super::Result<bool> {
        let talk = self
            .talk_service
            .find_by_id(&msg.talk_id)
            .await
            .map_err(|e| match e {
                talk::Error::NotFound(_) => message::Error::NotFound(Some(msg._id.clone())),
                e => message::Error::Unexpected(e.to_string()),
            })?;

        if let Some(last_message) = talk.last_message {
            return Ok(last_message.id == msg._id);
        }

        Ok(false)
    }
}

impl MessageService {
    async fn notify_new(&self, talk_id: &talk::Id, owner: &user::Sub, msgs: &[Message]) {
        match self.talk_service.find_recipients(talk_id, owner).await {
            Ok(recipients) => {
                let msg_evts: Vec<event::Message> = msgs
                    .iter()
                    .map(|m| event::Message::New(m.clone()))
                    .collect();

                for r in recipients {
                    self.event_service
                        .publish_all(&event::Subject::Messages(&r, talk_id), &msg_evts)
                        .await;
                }
            }
            Err(e) => error!("could not find talk recipients: {e:?}"),
        };
    }

    async fn notify_updated(&self, msg: &Message) {
        let talk_id = &msg.talk_id;
        let sub = &msg.owner;

        match self.talk_service.find_recipients(talk_id, sub).await {
            Ok(recipients) => {
                for r in recipients {
                    self.event_service
                        .publish(
                            &event::Subject::Messages(&r, talk_id),
                            &event::Message::Updated {
                                msg: msg.clone(),
                                logged_sub: sub.clone(),
                            },
                        )
                        .await;
                }
            }
            Err(e) => error!("could not find talk recipients: {e:?}"),
        };
    }

    async fn notify_deleted(&self, msg: &Message) {
        let talk_id = &msg.talk_id;
        let sub = &msg.owner;

        match self.talk_service.find_recipients(talk_id, sub).await {
            Ok(recipients) => {
                for r in recipients {
                    self.event_service
                        .publish(
                            &event::Subject::Messages(&r, talk_id),
                            &event::Message::Deleted(msg._id.clone()),
                        )
                        .await;
                }
            }
            Err(e) => error!("could not find talk recipients: {e:?}"),
        };
    }
}

fn split_message(splitter: &TextSplitter<Characters>, msg: &Message) -> Vec<Message> {
    let chunks = splitter.chunks(&msg.text);

    chunks
        .map(|text| msg.with_random_id().with_text(text))
        .collect::<Vec<Message>>()
}
