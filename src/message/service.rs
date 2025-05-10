use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error};
use text_splitter::{Characters, TextSplitter};

use crate::user::Sub;
use crate::{auth, event, message, talk};

use super::Repository;
use super::model::Message;

const MAX_MESSAGE_LENGTH: usize = 1000;

#[async_trait]
pub trait MessageService {
    async fn create(&self, msg: &Message) -> super::Result<Vec<Message>>;

    async fn find_by_id(&self, id: &message::Id) -> super::Result<Message>;

    async fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>>;

    async fn update(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
        text: &str,
    ) -> super::Result<Message>;

    async fn delete(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
    ) -> super::Result<Option<Message>>;

    async fn find_by_talk_id_and_params(
        &self,
        auth_sub: &Sub,
        talk_id: &talk::Id,
        limit: Option<i64>,
        end_time: Option<i64>,
    ) -> super::Result<(Vec<Message>, usize)>;

    async fn mark_as_seen(&self, auth_sub: &Sub, msgs: &[Message]) -> super::Result<usize>;

    async fn is_last_message(&self, msg: &Message) -> super::Result<bool>;
}

#[derive(Clone)]
pub struct MessageServiceImpl {
    repo: Repository,
    talk_service: talk::Service,
    talk_validator: talk::Validator,
    event_service: event::Service,
    splitter: Arc<TextSplitter<Characters>>,
}

impl MessageServiceImpl {
    pub fn new(
        repo: Repository,
        talk_service: talk::Service,
        talk_validator: talk::Validator,
        event_service: event::Service,
    ) -> Self {
        Self {
            repo,
            talk_service,
            talk_validator,
            event_service,
            splitter: Arc::new(TextSplitter::new(MAX_MESSAGE_LENGTH)),
        }
    }
}

#[async_trait]
impl MessageService for MessageServiceImpl {
    async fn create(&self, msg: &Message) -> super::Result<Vec<Message>> {
        if msg.text().is_empty() {
            return Err(super::Error::EmptyText);
        }

        let msgs = match msg.text().len() {
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

        self.notify_new(msg.talk_id(), msg.owner(), &msgs).await;

        Ok(msgs)
    }

    async fn find_by_id(&self, id: &message::Id) -> super::Result<Message> {
        self.repo.find_by_id(id).await
    }

    async fn find_most_recent(&self, talk_id: &talk::Id) -> super::Result<Option<Message>> {
        self.repo.find_most_recent(talk_id).await
    }

    async fn update(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
        text: &str,
    ) -> super::Result<Message> {
        let msg = self.repo.find_by_id(id).await?;

        if msg.owner().ne(auth_user.sub()) {
            return Err(super::Error::NotOwner);
        }

        self.repo.update(id, text).await?;
        let msg = msg.with_text(text);
        self.notify_updated(&msg).await;

        Ok(msg)
    }

    async fn delete(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
    ) -> super::Result<Option<Message>> {
        let msg = self.repo.find_by_id(id).await?;
        let talk_id = msg.talk_id();

        self.talk_validator
            .check_member(talk_id, auth_user)
            .await
            .map_err(|_| super::Error::NotOwner)?;

        if msg.owner().ne(auth_user.sub()) {
            return Err(super::Error::NotOwner);
        }

        if self.repo.delete(id).await? {
            self.notify_deleted(&msg).await;
            return Ok(Some(msg));
        }

        Ok(None)
    }

    // This method is designed to be callen when recipient requests messages related to selected talk.
    // It also marks all messages as seen where auth user is recipient.
    // Due to this side effect consider using other methods for read-only messages retrieval.
    async fn find_by_talk_id_and_params(
        &self,
        auth_sub: &Sub,
        talk_id: &talk::Id,
        limit: Option<i64>,
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

        let seen_qty = self.mark_as_seen(auth_sub, &msgs).await?;

        Ok((msgs, seen_qty))
    }

    async fn mark_as_seen(&self, auth_sub: &Sub, msgs: &[Message]) -> super::Result<usize> {
        if msgs.is_empty() {
            debug!("attempting to mark as seen but messages list is empty");
            return Ok(0);
        }

        let anothers_messages = msgs
            .iter()
            .filter(|msg| msg.owner().ne(auth_sub))
            .collect::<Vec<_>>();

        if anothers_messages.is_empty() {
            debug!("all messages belong to authenticated user, skipping mark as seen");
            return Ok(0);
        }

        let unseen_msgs = anothers_messages
            .into_iter()
            .filter(|msg| !msg.seen())
            .collect::<Vec<_>>();

        if unseen_msgs.is_empty() {
            debug!("all messages are already seen, skipping mark as seen");
            return Ok(0);
        }

        let unseen_ids = unseen_msgs
            .iter()
            .map(|msg| msg.id().clone())
            .collect::<Vec<_>>();

        self.repo.mark_as_seen(&unseen_ids).await?;

        let msg_evts = unseen_msgs
            .iter()
            .map(|m| event::Message::Seen((*m).clone()))
            .map(bytes::Bytes::from)
            .collect::<Vec<_>>();

        let subjects = unseen_msgs
            .iter()
            .map(|msg| event::Subject::Messages(msg.owner(), msg.talk_id()))
            .collect::<Vec<_>>();

        let seen_qty = msg_evts.len();
        self.event_service
            .broadcast_many(&subjects, &msg_evts)
            .await;

        Ok(seen_qty)
    }

    async fn is_last_message(&self, msg: &Message) -> super::Result<bool> {
        let talk = self
            .talk_service
            .find_by_id(msg.talk_id())
            .await
            .map_err(|e| match e {
                talk::Error::NotFound(_) => message::Error::NotFound(Some(msg.id().clone())),
                e => message::Error::Unexpected(e.to_string()),
            })?;

        if let Some(last_message) = talk.last_message() {
            return Ok(last_message.id().eq(msg.id()));
        }

        Ok(false)
    }
}

impl MessageServiceImpl {
    async fn notify_new(&self, talk_id: &talk::Id, owner: &Sub, msgs: &[Message]) {
        match self.talk_service.find_recipients(talk_id, owner).await {
            Ok(recipients) => {
                let msg_evts = msgs
                    .iter()
                    .map(|m| event::Message::New(m.clone()))
                    .map(bytes::Bytes::from)
                    .collect::<Vec<_>>();

                let subjects = recipients
                    .iter()
                    .map(|r| event::Subject::Messages(r, talk_id))
                    .collect::<Vec<_>>();

                self.event_service
                    .broadcast_many(&subjects, &msg_evts)
                    .await;
            }
            Err(e) => error!("could not find talk recipients: {e:?}"),
        }
    }

    async fn notify_updated(&self, msg: &Message) {
        let talk_id = msg.talk_id();
        let sub = msg.owner();

        match self.talk_service.find_recipients(talk_id, sub).await {
            Ok(recipients) => {
                let subjects = recipients
                    .iter()
                    .map(|r| event::Subject::Messages(r, talk_id))
                    .collect::<Vec<_>>();

                self.event_service
                    .broadcast(
                        &subjects,
                        event::Message::Updated {
                            msg: msg.clone(),
                            auth_sub: sub.clone(),
                        }
                        .into(),
                    )
                    .await;
            }
            Err(e) => error!("could not find talk recipients: {e:?}"),
        }
    }

    async fn notify_deleted(&self, msg: &Message) {
        let talk_id = msg.talk_id();
        let sub = msg.owner();

        match self.talk_service.find_recipients(talk_id, sub).await {
            Ok(recipients) => {
                let subjects = recipients
                    .iter()
                    .map(|r| event::Subject::Messages(r, talk_id))
                    .collect::<Vec<_>>();

                self.event_service
                    .broadcast(&subjects, event::Message::Deleted(msg.id().clone()).into())
                    .await;
            }
            Err(e) => error!("could not find talk recipients: {e:?}"),
        }
    }
}

fn split_message(splitter: &TextSplitter<Characters>, msg: &Message) -> Vec<Message> {
    let chunks = splitter.chunks(msg.text());

    chunks
        .map(|text| msg.with_random_id().with_text(text))
        .collect::<Vec<Message>>()
}
