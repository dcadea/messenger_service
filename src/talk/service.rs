use std::collections::HashSet;

use async_trait::async_trait;
use futures::future::join_all;
use log::error;

use super::model::{Details, DetailsDto, Talk, TalkDto};
use super::{Kind, Repository, Validator};
use crate::integration::{cache, storage};
use crate::message::model::LastMessage;
use crate::user::Sub;
use crate::{auth, contact, event, message, talk, user};

#[async_trait]
pub trait TalkService {
    async fn create_chat(&self, auth_sub: &Sub, recipient: &Sub) -> super::Result<TalkDto>;

    async fn create_group(
        &self,
        auth_sub: &Sub,
        name: &str,
        members: &[Sub],
    ) -> super::Result<TalkDto>;

    async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk>;

    async fn find_by_id_and_sub(&self, id: &talk::Id, auth_sub: &Sub) -> super::Result<TalkDto>;

    async fn find_all_by_kind(
        &self,
        auth_user: &auth::User,
        kind: &Kind,
    ) -> super::Result<Vec<TalkDto>>;

    async fn find_recipients(
        &self,
        talk_id: &talk::Id,
        auth_sub: &Sub,
    ) -> super::Result<HashSet<Sub>>;

    async fn delete(&self, id: &talk::Id, auth_user: &auth::User) -> super::Result<()>;

    async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()>;

    async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()>;
}

#[derive(Clone)]
pub struct TalkServiceImpl {
    repo: Repository,
    validator: Validator,
    user_service: user::Service,
    contact_service: contact::Service,
    event_service: event::Service,
    message_repo: message::Repository,
    redis: cache::Redis,
    s3: storage::S3,
}

impl TalkServiceImpl {
    pub fn new(
        repo: Repository,
        validator: Validator,
        user_service: user::Service,
        contact_service: contact::Service,
        event_service: event::Service,
        message_repo: message::Repository,
        redis: cache::Redis,
        s3: storage::S3,
    ) -> Self {
        Self {
            repo,
            validator,
            user_service,
            contact_service,
            event_service,
            message_repo,
            redis,
            s3,
        }
    }
}

#[async_trait]
impl TalkService for TalkServiceImpl {
    async fn create_chat(&self, auth_sub: &Sub, recipient: &Sub) -> super::Result<TalkDto> {
        assert_ne!(auth_sub, recipient);

        let members = [auth_sub.clone(), recipient.clone()];
        if self.repo.exists(&members).await? {
            return Err(talk::Error::AlreadyExists);
        }

        let contact = self
            .contact_service
            .find(auth_sub, recipient)
            .await
            .map_err(|e| {
                error!("could not create contact: {e:?}");
                talk::Error::NotCreated
            })?;

        if contact.is_none_or(|c| !c.is_accepted()) {
            return Err(talk::Error::UnsupportedStatus);
        }

        let talk = Talk::from(Details::Chat { members });
        self.repo.create(&talk).await?;

        let talk_dto = self.talk_to_dto(talk, auth_sub).await;

        self.event_service
            .publish(
                &event::Subject::Notifications(recipient),
                event::Notification::NewTalk(talk_dto.clone()).into(),
            )
            .await;

        Ok(talk_dto)
    }

    async fn create_group(
        &self,
        auth_sub: &Sub,
        name: &str,
        members: &[Sub],
    ) -> super::Result<TalkDto> {
        assert!(members.contains(auth_sub));

        if name.is_empty() {
            return Err(talk::Error::MissingName);
        }

        if members.len() < 3 {
            return Err(talk::Error::NotEnoughMembers(members.len()));
        }

        for m in members {
            let exists = self.user_service.exists(m).await?;

            if !exists {
                return Err(talk::Error::NonExistingUser(m.clone()));
            }
        }

        let id = talk::Id::random();
        let picture = self.s3.generate_image(id.as_str()).await?;

        let talk = Talk::new(
            id,
            Details::Group {
                name: name.into(),
                picture,
                owner: auth_sub.clone(),
                members: members.into(),
            },
        );
        self.repo.create(&talk).await?;

        let talk_dto = self.talk_to_dto(talk, auth_sub).await;

        for m in members {
            if m.eq(auth_sub) {
                continue;
            }

            self.event_service
                .publish(
                    &event::Subject::Notifications(m),
                    event::Notification::NewTalk(talk_dto.clone()).into(),
                )
                .await;
        }

        Ok(talk_dto)
    }

    async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk> {
        let talk = self.repo.find_by_id(id).await?;
        Ok(talk)
    }

    async fn find_by_id_and_sub(&self, id: &talk::Id, auth_sub: &Sub) -> super::Result<TalkDto> {
        let talk = self.repo.find_by_id_and_sub(id, auth_sub).await?;
        let dto = self.talk_to_dto(talk, auth_sub).await;
        Ok(dto)
    }

    async fn find_all_by_kind(
        &self,
        auth_user: &auth::User,
        kind: &Kind,
    ) -> super::Result<Vec<TalkDto>> {
        let auth_sub = auth_user.sub();
        let talks = self.repo.find_by_sub_and_kind(auth_sub, kind).await?;

        let talk_dtos = join_all(
            talks
                .into_iter()
                .map(|t| async { self.talk_to_dto(t, auth_sub).await }),
        )
        .await;

        Ok(talk_dtos)
    }

    async fn find_recipients(
        &self,
        talk_id: &talk::Id,
        auth_sub: &Sub,
    ) -> super::Result<HashSet<Sub>> {
        let recipients = {
            let mut r = find_members(&self.redis, self.repo.clone(), talk_id).await?;
            r.remove(auth_sub);
            r
        };

        Ok(recipients)
    }

    async fn delete(&self, id: &talk::Id, auth_user: &auth::User) -> super::Result<()> {
        self.validator.check_member(id, auth_user).await?;

        // TODO: check if the user is the owner of the group
        self.repo.delete(id).await?;
        if let Err(e) = self.message_repo.delete_by_talk_id(id).await {
            error!("failed to delete talk: {e:?}");
            return Err(talk::Error::NotDeleted);
            // TODO: tx rollback?
        }

        Ok(())
    }

    async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()> {
        self.repo.update_last_message(id, msg).await?;

        if let Some(last_msg) = msg {
            let recipients = self.find_recipients(id, last_msg.owner()).await?;
            let subjects = recipients
                .iter()
                .map(event::Subject::Notifications)
                .collect::<Vec<_>>();

            self.event_service
                .broadcast(
                    &subjects,
                    event::Notification::NewMessage {
                        talk_id: id.clone(),
                        last_message: last_msg.clone(),
                    }
                    .into(),
                )
                .await;
        }
        Ok(())
    }

    async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()> {
        self.repo.mark_as_seen(id).await
    }
}

impl TalkServiceImpl {
    async fn talk_to_dto(&self, t: Talk, auth_sub: &Sub) -> TalkDto {
        let (name, picture, details) = match t.details().clone() {
            Details::Chat { members } => {
                assert!(members.contains(auth_sub));

                let (sender, recipient) = {
                    if members[0].eq(auth_sub) {
                        (members[0].clone(), members[1].clone())
                    } else {
                        (members[1].clone(), members[0].clone())
                    }
                };

                let r = self
                    .user_service
                    .find_one(&recipient)
                    .await
                    .expect("recipient info should be present");

                (
                    r.name().to_string(),
                    r.picture().to_string(),
                    DetailsDto::Chat { sender, recipient },
                )
            }
            Details::Group {
                name,
                picture,
                owner,
                ..
            } => (
                name,
                picture,
                DetailsDto::Group {
                    owner,
                    sender: auth_sub.clone(),
                },
            ),
        };

        TalkDto::new(
            t.id().clone(),
            picture,
            name,
            details,
            t.last_message().cloned(),
        )
    }
}

#[async_trait]
pub trait TalkValidator {
    async fn check_member(&self, talk_id: &talk::Id, auth_user: &auth::User) -> super::Result<()>;
}

#[derive(Clone)]
pub struct TalkValidatorImpl {
    repo: Repository,
    redis: cache::Redis,
}

impl TalkValidatorImpl {
    pub fn new(repo: Repository, redis: cache::Redis) -> Self {
        Self { repo, redis }
    }
}

#[async_trait]
impl TalkValidator for TalkValidatorImpl {
    async fn check_member(&self, talk_id: &talk::Id, auth_user: &auth::User) -> super::Result<()> {
        let members = find_members(&self.redis, self.repo.clone(), talk_id).await?;
        let belongs_to_talk = members.contains(auth_user.sub());

        if !belongs_to_talk {
            return Err(talk::Error::NotMember);
        }

        Ok(())
    }
}

async fn find_members(
    redis: &cache::Redis,
    repo: Repository,
    talk_id: &talk::Id,
) -> super::Result<HashSet<Sub>> {
    let talk_key = cache::Key::Talk(talk_id);
    let members = redis.smembers::<HashSet<Sub>>(talk_key.clone()).await;

    match members {
        Some(m) if !m.is_empty() => Ok(m),
        _ => {
            let talk = repo.find_by_id(talk_id).await?;
            let m = match talk.details().clone() {
                Details::Chat { members } => members.into(),
                Details::Group { members, .. } => HashSet::from_iter(members),
            };

            redis.sadd(talk_key.clone(), &m).await;
            redis.expire(talk_key).await;

            Ok(m)
        }
    }
}
