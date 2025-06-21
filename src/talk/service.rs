use std::collections::HashSet;

use async_trait::async_trait;
use futures::TryFutureExt;
use log::error;

use super::model::{ChatTalk, Details, DetailsDto, GroupTalk, Talk, TalkDto};
use super::{Kind, Repository, Validator};
use crate::integration::storage::Blob;
use crate::integration::{cache, storage};
use crate::message::model::LastMessage;
use crate::talk::Picture;
use crate::talk::model::NewTalk;
use crate::{auth, contact, event, talk, user};

#[async_trait]
pub trait TalkService {
    async fn create_chat(&self, auth_id: &user::Id, recipient: &user::Id)
    -> super::Result<TalkDto>;

    async fn create_group(
        &self,
        auth_id: &user::Id,
        name: &str,
        members: &[user::Id],
    ) -> super::Result<TalkDto>;

    async fn find_by_id_and_user_id(
        &self,
        id: &talk::Id,
        user_id: &user::Id,
    ) -> super::Result<TalkDto>;

    async fn find_all_by_kind(
        &self,
        auth_user: &auth::User,
        kind: &Kind,
    ) -> super::Result<Vec<TalkDto>>;

    async fn find_recipients(
        &self,
        talk_id: &talk::Id,
        auth_id: &user::Id,
    ) -> super::Result<HashSet<user::Id>>;

    async fn delete(&self, id: &talk::Id, auth_user: &auth::User) -> super::Result<()>;

    async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()>;

    fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()>;
}

#[derive(Clone)]
pub struct TalkServiceImpl {
    repo: Repository,
    validator: Validator,
    user_service: user::Service,
    contact_service: contact::Service,
    event_service: event::Service,
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
        redis: cache::Redis,
        s3: storage::S3,
    ) -> Self {
        Self {
            repo,
            validator,
            user_service,
            contact_service,
            event_service,
            redis,
            s3,
        }
    }
}

#[async_trait]
impl TalkService for TalkServiceImpl {
    async fn create_chat(
        &self,
        auth_id: &user::Id,
        recipient: &user::Id,
    ) -> super::Result<TalkDto> {
        assert_ne!(auth_id, recipient);

        let members = [auth_id.clone(), recipient.clone()];
        if self.repo.exists(&members)? {
            return Err(talk::Error::AlreadyExists);
        }

        let contact = self
            .contact_service
            .find(auth_id, recipient)
            .await
            .map_err(|e| {
                error!("could not find contact: {e:?}");
                talk::Error::NotCreated
            })?;

        if contact.is_none_or(|c| !c.is_accepted()) {
            return Err(talk::Error::UnsupportedStatus);
        }

        let id = self
            .repo
            .create(&NewTalk::new(&Details::Chat { members }))?;

        let r = self.user_service.find_one(&recipient).await?;
        let talk_dto = TalkDto::new(
            id,
            Picture::from(r.picture().clone()),
            r.name(),
            DetailsDto::Chat {
                sender: auth_id.clone(),
                recipient: r.id().clone(),
            },
            None,
        );

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
        auth_id: &user::Id,
        name: &str,
        members: &[user::Id],
    ) -> super::Result<TalkDto> {
        assert!(members.contains(auth_id));

        if name.is_empty() {
            return Err(talk::Error::MissingName);
        }

        if members.len() < 3 {
            return Err(talk::Error::NotEnoughMembers(members.len()));
        }

        for m in members {
            let exists = self.user_service.exists(m)?;

            if !exists {
                return Err(talk::Error::NonExistingUser(m.clone()));
            }
        }

        let details = Details::Group {
            name: name.into(),
            owner: auth_id.clone(),
            members: members.into(),
        };

        let id = self.repo.create(&NewTalk::new(&details))?;
        self.s3
            .generate(Blob::Png(&id.0.to_string()))
            .map_err(talk::Error::from)
            .await?;

        let talk_dto = TalkDto::new(
            id.clone(),
            Picture::from(id.clone()),
            name,
            DetailsDto::Group {
                owner: auth_id.clone(),
                sender: auth_id.clone(),
            },
            None,
        );

        for m in members {
            if m.eq(auth_id) {
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

    async fn find_by_id_and_user_id(
        &self,
        id: &talk::Id,
        auth_id: &user::Id,
    ) -> super::Result<TalkDto> {
        let _talk = self.repo.find_by_id_and_user_id(id, auth_id)?;
        // let dto = self.talk_to_dto(talk, auth_id).await;
        // Ok(dto)
        todo!()
    }

    async fn find_all_by_kind(
        &self,
        auth_user: &auth::User,
        kind: &Kind,
    ) -> super::Result<Vec<TalkDto>> {
        let auth_id = auth_user.id();

        let talk_dtos: Vec<TalkDto> = match kind {
            Kind::Chat => self
                .repo
                .find_chats_by_user_id(auth_id)?
                .iter()
                .map(|c| self.chat_to_dto(c, auth_id))
                .collect(),
            Kind::Group => self
                .repo
                .find_groups_by_user_id(auth_id)?
                .iter()
                .map(|g| self.group_to_dto(g, auth_id))
                .collect(),
        };

        Ok(talk_dtos)
    }

    async fn find_recipients(
        &self,
        talk_id: &talk::Id,
        auth_id: &user::Id,
    ) -> super::Result<HashSet<user::Id>> {
        let recipients = {
            let mut r = find_members(&self.redis, self.repo.clone(), talk_id).await?;
            r.remove(auth_id);
            r
        };

        Ok(recipients)
    }

    async fn delete(&self, id: &talk::Id, auth_user: &auth::User) -> super::Result<()> {
        self.validator.check_member(id, auth_user).await?;

        self.repo.delete(id)?;

        Ok(())
    }

    async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()> {
        self.repo.update_last_message(id, msg)?;

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

    fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()> {
        self.repo.mark_as_seen(id)
    }
}

impl TalkServiceImpl {
    fn chat_to_dto(&self, c: &ChatTalk, auth_id: &user::Id) -> TalkDto {
        let lm = c.last_message_id().map(|_| todo!("fetch last message"));

        TalkDto::new(
            c.id().clone(),
            Picture::from(user::Picture::try_from(c.picture()).unwrap()), // FIXME
            c.name(),
            DetailsDto::Chat {
                sender: auth_id.clone(),
                recipient: c.recipient().clone(),
            },
            lm,
        )
    }

    fn group_to_dto(&self, g: &GroupTalk, auth_id: &user::Id) -> TalkDto {
        let lm = g.last_message_id().map(|_| todo!("fetch last message"));

        TalkDto::new(
            g.id().clone(),
            Picture::from(g.id().clone()),
            g.name(),
            DetailsDto::Group {
                owner: g.owner().clone(),
                sender: auth_id.clone(),
            },
            lm,
        )
    }

    async fn _talk_to_dto(&self, _t: Talk, _auth_id: &user::Id) -> TalkDto {
        // let (name, picture, details) = match t.details().clone() {
        //     Details::Chat { members } => {
        //         assert!(members.contains(auth_id));

        //         let (sender, recipient) = {
        //             if members[0].eq(auth_id) {
        //                 (members[0].clone(), members[1].clone())
        //             } else {
        //                 (members[1].clone(), members[0].clone())
        //             }
        //         };

        //         let r = self
        //             .user_service
        //             .find_one(&recipient)
        //             .await
        //             .expect("recipient info should be present");

        //         (
        //             r.name().to_string(),
        //             Picture::from(r.picture().clone()),
        //             DetailsDto::Chat { sender, recipient },
        //         )
        //     }
        //     Details::Group { name, owner, .. } => (
        //         name,
        //         Picture::from(t.id().clone()),
        //         DetailsDto::Group {
        //             owner,
        //             sender: auth_id.clone(),
        //         },
        //     ),
        // };

        // TalkDto::new(
        //     t.id().clone(),
        //     picture,
        //     name,
        //     details,
        //     t.last_message().cloned(),
        // )
        todo!()
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
        let belongs_to_talk = members.contains(auth_user.id());

        if !belongs_to_talk {
            return Err(talk::Error::NotMember);
        }

        Ok(())
    }
}

async fn find_members(
    redis: &cache::Redis,
    _repo: Repository,
    talk_id: &talk::Id,
) -> super::Result<HashSet<user::Id>> {
    let talk_key = cache::Key::Talk(talk_id);
    let _members = redis.smembers::<HashSet<user::Id>>(talk_key.clone()).await;

    // TODO: write a proper db query
    // match members {
    //     Some(m) if !m.is_empty() => Ok(m),
    //     _ => {
    //         let talk = repo.find_by_id(talk_id)?;
    //         let m = match talk.details().clone() {
    //             Details::Chat { members } => members.into(),
    //             Details::Group { members, .. } => HashSet::from_iter(members),
    //         };

    //         redis.sadd(talk_key.clone(), &m).await;
    //         redis.expire(talk_key).await;

    //         Ok(m)
    //     }
    // }
    todo!()
}
