use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use log::{debug, error};
use text_splitter::{Characters, TextSplitter};

use crate::message::model::NewMessage;
use crate::user::{self};
use crate::{auth, event, message, talk};

use super::Repository;
use super::model::MessageDto;

const MAX_MESSAGE_LENGTH: usize = 1000;

#[async_trait]
pub trait MessageService {
    async fn create(
        &self,
        talk_id: &talk::Id,
        auth_user: &auth::User,
        text: &str,
    ) -> super::Result<Vec<MessageDto>>;

    fn find_by_id(&self, auth_user: &auth::User, id: &message::Id) -> super::Result<MessageDto>;

    async fn update(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
        text: &str,
    ) -> super::Result<MessageDto>;

    async fn delete(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
    ) -> super::Result<Option<MessageDto>>;

    async fn find_by_talk_id_and_params(
        &self,
        auth_user: &auth::User,
        talk_id: &talk::Id,
        limit: Option<i64>,
        end_time: Option<NaiveDateTime>,
    ) -> super::Result<Vec<MessageDto>>;

    async fn mark_as_seen(&self, auth_id: &user::Id, msgs: &[MessageDto]) -> super::Result<usize>;
}

#[derive(Clone)]
pub struct MessageServiceImpl {
    repo: Repository,
    user_service: user::Service,
    event_service: event::Service,
    splitter: Arc<TextSplitter<Characters>>,
}

impl MessageServiceImpl {
    pub fn new(
        repo: Repository,
        user_service: user::Service,
        event_service: event::Service,
    ) -> Self {
        Self {
            repo,
            user_service,
            event_service,
            splitter: Arc::new(TextSplitter::new(MAX_MESSAGE_LENGTH)),
        }
    }
}

#[async_trait]
impl MessageService for MessageServiceImpl {
    async fn create(
        &self,
        talk_id: &talk::Id,
        auth_user: &auth::User,
        content: &str,
    ) -> super::Result<Vec<MessageDto>> {
        let content: String = content.into();
        if content.is_empty() {
            return Err(super::Error::EmptyContent);
        }

        let auth_id = auth_user.id();

        let msgs = match content.len() {
            text_length if text_length <= MAX_MESSAGE_LENGTH => {
                let new_msg = NewMessage::new(talk_id, auth_id, &content);
                let msg = self.repo.insert(&new_msg)?;
                vec![msg]
            }
            _ => {
                let msgs = split_content(&self.splitter, talk_id, auth_id, &content);
                self.repo.insert_many(&msgs)?
            }
        };

        let msgs = msgs
            .into_iter()
            .map(MessageDto::from)
            .collect::<Vec<MessageDto>>();

        self.notify_new(talk_id, auth_id, &msgs).await;

        Ok(msgs)
    }

    fn find_by_id(&self, auth_user: &auth::User, id: &message::Id) -> super::Result<MessageDto> {
        self.repo
            .find_by_id(auth_user.id(), id)
            .map(MessageDto::from)
    }

    async fn update(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
        text: &str,
    ) -> super::Result<MessageDto> {
        let msg = self
            .repo
            .find_by_id(auth_user.id(), id)
            .map(MessageDto::from)?;

        if self.repo.update(auth_user.id(), id, text)? {
            let msg = msg.with_text(text);
            self.notify_updated(&msg).await;
            return Ok(msg);
        }

        Ok(msg)
    }

    async fn delete(
        &self,
        auth_user: &auth::User,
        id: &message::Id,
    ) -> super::Result<Option<MessageDto>> {
        let msg = self
            .repo
            .find_by_id(auth_user.id(), id)
            .map(MessageDto::from)?;

        if self.repo.delete(auth_user.id(), id)? {
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
        auth_user: &auth::User,
        talk_id: &talk::Id,
        limit: Option<i64>,
        end_time: Option<NaiveDateTime>,
    ) -> super::Result<Vec<MessageDto>> {
        let msgs = match (limit, end_time) {
            (None, None) => self.repo.find_by_talk_id(talk_id),
            (Some(limit), None) => self.repo.find_by_talk_id_limited(talk_id, limit),
            (None, Some(end_time)) => self.repo.find_by_talk_id_before(talk_id, end_time),
            (Some(limit), Some(end_time)) => self
                .repo
                .find_by_talk_id_limited_before(talk_id, limit, end_time),
        }?;

        let msgs = msgs
            .into_iter()
            .map(MessageDto::from)
            .collect::<Vec<MessageDto>>();

        self.mark_as_seen(auth_user.id(), &msgs).await?;

        Ok(msgs)
    }

    async fn mark_as_seen(&self, auth_id: &user::Id, msgs: &[MessageDto]) -> super::Result<usize> {
        if msgs.is_empty() {
            debug!("attempting to mark as seen but messages list is empty");
            return Ok(0);
        }

        let anothers_messages = msgs
            .iter()
            .filter(|msg| msg.owner().ne(auth_id))
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

        self.repo.mark_as_seen(&unseen_ids)?;

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
}

impl MessageServiceImpl {
    async fn notify_new(&self, talk_id: &talk::Id, owner: &user::Id, msgs: &[MessageDto]) {
        match self.find_recipients(talk_id, owner).await {
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

    async fn notify_updated(&self, msg: &MessageDto) {
        let talk_id = msg.talk_id();
        let owner = msg.owner();

        match self.find_recipients(talk_id, owner).await {
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
                            auth_id: owner.clone(),
                        }
                        .into(),
                    )
                    .await;
            }
            Err(e) => error!("could not find talk recipients: {e:?}"),
        }
    }

    async fn notify_deleted(&self, msg: &MessageDto) {
        let talk_id = msg.talk_id();
        let owner = msg.owner();

        match self.find_recipients(talk_id, owner).await {
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

    async fn find_recipients(
        &self,
        talk_id: &talk::Id,
        exclude: &user::Id,
    ) -> super::Result<HashSet<user::Id>> {
        let recipients = {
            let mut r = self.user_service.find_members(talk_id).await?;
            r.remove(exclude);
            r
        };

        Ok(recipients)
    }
}

fn split_content<'a>(
    splitter: &'a TextSplitter<Characters>,
    talk_id: &'a talk::Id,
    owner: &'a user::Id,
    content: &'a str,
) -> Vec<NewMessage<'a>> {
    let chunks = splitter.chunks(content);

    chunks
        .map(|chunk| NewMessage::new(talk_id, owner, chunk))
        .collect::<Vec<NewMessage<'a>>>()
}
