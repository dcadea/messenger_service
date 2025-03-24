use std::collections::HashSet;
use std::sync::Arc;

use futures::future::join_all;
use log::error;

use super::model::{Details, DetailsDto, Talk, TalkDto};
use super::repository::TalkRepository;
use crate::event::service::EventService;
use crate::integration::cache;
use crate::message::model::LastMessage;
use crate::user::model::UserInfo;
use crate::{event, message, talk, user};

#[derive(Clone)]
pub struct TalkService {
    repo: Arc<TalkRepository>,
    validator: Arc<TalkValidator>,
    user_service: user::Service,
    event_service: Arc<EventService>,
    message_repo: message::Repository,
    redis: cache::Redis,
}

impl TalkService {
    pub fn new(
        repo: TalkRepository,
        validator: TalkValidator,
        user_service: user::Service,
        event_service: EventService,
        message_repo: message::Repository,
        redis: cache::Redis,
    ) -> Self {
        Self {
            repo: Arc::new(repo),
            validator: Arc::new(validator),
            user_service,
            event_service: Arc::new(event_service),
            message_repo,
            redis,
        }
    }
}

impl TalkService {
    pub async fn create_chat(
        &self,
        logged_sub: &user::Sub,
        recipient: &user::Sub,
    ) -> super::Result<TalkDto> {
        assert_ne!(logged_sub, recipient);

        let members = [logged_sub.clone(), recipient.clone()];
        if self.repo.exists(&members).await? {
            return Err(talk::Error::AlreadyExists);
        }

        // TODO: revisit this
        // if let Err(e) = self.user_service.create_contact(&members).await {
        //     error!("could not create contact: {e:?}");
        //     return Err(talk::Error::NotCreated);
        // }

        let talk = Talk::new(Details::Chat { members });
        self.repo.create(talk.clone()).await?;

        let talk_dto = self.talk_to_dto(talk, logged_sub).await;

        self.event_service
            .publish(
                &event::Subject::Notifications(recipient),
                &event::Notification::NewTalk(talk_dto.clone()),
            )
            .await;

        Ok(talk_dto)
    }

    pub async fn create_group(
        &self,
        logged_sub: &user::Sub,
        name: &str,
        members: &[user::Sub],
    ) -> super::Result<TalkDto> {
        assert!(members.contains(logged_sub));

        if members.len() < 3 {
            return Err(talk::Error::NotEnoughMembers(members.len()));
        }

        let talk = Talk::new(Details::Group {
            name: name.into(),
            picture: "".into(), // TODO: https://crates.io/crates/identicon-rs
            owner: logged_sub.clone(),
            members: members.into(),
        });
        self.repo.create(talk.clone()).await?;

        let talk_dto = self.talk_to_dto(talk, logged_sub).await;

        for m in members {
            if m.eq(logged_sub) {
                continue;
            }

            self.event_service
                .publish(
                    &event::Subject::Notifications(m),
                    &event::Notification::NewTalk(talk_dto.clone()),
                )
                .await;
        }

        Ok(talk_dto)
    }

    pub async fn find_by_id(&self, id: &talk::Id) -> super::Result<Talk> {
        let talk = self.repo.find_by_id(id).await?;
        Ok(talk)
    }

    pub async fn find_by_id_and_sub(
        &self,
        id: &talk::Id,
        logged_sub: &user::Sub,
    ) -> super::Result<TalkDto> {
        let talk = self.repo.find_by_id_and_sub(id, logged_sub).await?;
        let dto = self.talk_to_dto(talk, logged_sub).await;
        Ok(dto)
    }

    pub async fn find_all(&self, user_info: &UserInfo) -> super::Result<Vec<TalkDto>> {
        let sub = &user_info.sub;
        let talks = self.repo.find_by_sub(sub).await?;

        let talk_dtos = join_all(
            talks
                .into_iter()
                .map(|t| async { self.talk_to_dto(t, sub).await }),
        )
        .await;

        Ok(talk_dtos)
    }

    pub async fn find_recipients(
        &self,
        talk_id: &talk::Id,
        logged_sub: &user::Sub,
    ) -> super::Result<HashSet<user::Sub>> {
        let mut recipients = find_members(&self.redis, self.repo.clone(), talk_id).await?;
        recipients.remove(logged_sub);
        Ok(recipients)
    }

    pub async fn delete(&self, id: &talk::Id, logged_user: &UserInfo) -> super::Result<()> {
        let logged_sub = &logged_user.sub;
        self.validator.check_member(id, logged_sub).await?;

        // TODO: revisit this
        // let talk = self.find_by_id_and_sub(id, logged_sub).await?;
        // let contacts = [talk.sender, talk.recipient];
        // if let Err(e) = self.user_service.delete_contact(&contacts).await {
        //     error!("could not delete contact: {e:?}");
        //     return Err(talk::Error::NotDeleted);
        // }

        self.repo.delete(id).await?;
        if let Err(e) = self.message_repo.delete_by_talk_id(id).await {
            error!("failed to delete talk: {e:?}");
            return Err(talk::Error::NotDeleted);
            // TODO: tx rollback?
        }

        Ok(())
    }
}

impl TalkService {
    async fn talk_to_dto(&self, t: Talk, logged_sub: &user::Sub) -> TalkDto {
        let (name, picture, details) = match t.details {
            Details::Chat { members } => {
                assert!(members.contains(logged_sub));

                let (sender, recipient) = {
                    if members[0].eq(logged_sub) {
                        (members[0].clone(), members[1].clone())
                    } else {
                        (members[1].clone(), members[0].clone())
                    }
                };

                let r = self
                    .user_service
                    .find_user_info(&recipient)
                    .await
                    .expect("recipient info should be present");

                (r.name, r.picture, DetailsDto::Chat { sender, recipient })
            }
            Details::Group { name, picture, .. } => (name, picture, DetailsDto::Group),
        };

        TalkDto {
            id: t._id,
            picture,
            name,
            details,
            last_message: t.last_message,
        }
    }
}

impl TalkService {
    pub async fn update_last_message(
        &self,
        id: &talk::Id,
        msg: Option<&LastMessage>,
    ) -> super::Result<()> {
        self.repo.update_last_message(id, msg).await?;

        if let Some(last_msg) = msg {
            let recipients = self.find_recipients(id, &last_msg.owner).await?;

            for r in recipients {
                self.event_service
                    .publish(
                        &event::Subject::Notifications(&r),
                        &event::Notification::NewMessage {
                            talk_id: id.clone(),
                            last_message: last_msg.clone(),
                        },
                    )
                    .await;
            }
        }
        Ok(())
    }

    pub async fn mark_as_seen(&self, id: &talk::Id) -> super::Result<()> {
        self.repo.mark_as_seen(id).await
    }
}

#[derive(Clone)]
pub struct TalkValidator {
    repo: Arc<TalkRepository>,
    redis: cache::Redis,
}

impl TalkValidator {
    pub fn new(repo: TalkRepository, redis: cache::Redis) -> Self {
        Self {
            repo: Arc::new(repo),
            redis,
        }
    }
}

impl TalkValidator {
    pub async fn check_member(&self, talk_id: &talk::Id, sub: &user::Sub) -> super::Result<()> {
        let members = find_members(&self.redis, self.repo.clone(), talk_id).await?;
        let belongs_to_talk = members.contains(sub);

        if !belongs_to_talk {
            return Err(talk::Error::NotMember);
        }

        Ok(())
    }
}

async fn find_members(
    redis: &cache::Redis,
    repo: Arc<TalkRepository>,
    talk_id: &talk::Id,
) -> super::Result<HashSet<user::Sub>> {
    let talk_key = cache::Key::Talk(talk_id.to_owned());
    let members = redis.smembers::<HashSet<user::Sub>>(talk_key.clone()).await;

    match members {
        Some(m) if !m.is_empty() => Ok(m),
        _ => {
            let talk = repo.find_by_id(talk_id).await?;
            let members: HashSet<user::Sub> = match talk.details {
                Details::Chat { members } => members.into(),
                Details::Group { members, .. } => HashSet::from_iter(members),
            };

            redis.sadd(talk_key.clone(), &members).await;
            redis.expire(talk_key).await;

            Ok(members)
        }
    }
}
