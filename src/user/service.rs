use std::collections::HashSet;

use async_trait::async_trait;
use log::error;

use crate::integration::cache;
use crate::user::model::UserDto;
use crate::{auth, contact, event, talk, user};

use super::model::{NewUser, OnlineStatus};
use super::{Nickname, Picture, Repository, Sub};

#[async_trait]
pub trait UserService {
    fn project(&self, user_info: &auth::UserInfo) -> super::Result<user::Id>;

    async fn find_one(&self, id: &user::Id) -> super::Result<UserDto>;

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<UserDto>;

    async fn find_members(&self, talk_id: &talk::Id) -> super::Result<HashSet<user::Id>>;

    async fn check_member(&self, talk_id: &talk::Id, auth_user: &auth::User) -> super::Result<()>;

    async fn find_name(&self, id: &user::Id) -> super::Result<String>;

    async fn find_picture(&self, id: &user::Id) -> super::Result<Picture>;

    fn exists(&self, id: &user::Id) -> super::Result<bool>;

    fn search(&self, nickname: &Nickname, auth_user: &auth::User) -> super::Result<Vec<UserDto>>;

    async fn notify_online(&self, id: &user::Id);

    async fn notify_offline(&self, id: &user::Id);
}

#[derive(Clone)]
pub struct UserServiceImpl {
    repo: Repository,
    contact_service: contact::Service,
    event_service: event::Service,
    redis: cache::Redis,
}

impl UserServiceImpl {
    pub fn new(
        repo: Repository,
        contact_service: contact::Service,
        event_service: event::Service,
        redis: cache::Redis,
    ) -> Self {
        Self {
            repo,
            contact_service,
            event_service,
            redis,
        }
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    fn project(&self, u: &auth::UserInfo) -> super::Result<user::Id> {
        self.repo.create(&NewUser::from(u))
    }

    async fn find_one(&self, id: &user::Id) -> super::Result<UserDto> {
        if let Some(u) = self.find_cached(id).await {
            Ok(u)
        } else {
            let u = self.repo.find_by_id(id).map(UserDto::from)?;
            self.cache(&u).await;
            Ok(u)
        }
    }

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<UserDto> {
        let u = self.repo.find_by_sub(sub)?.map(UserDto::from);
        match u {
            Some(u) => {
                self.cache(&u).await;
                Ok(u)
            }
            None => Err(super::Error::NotFound(sub.clone())),
        }
    }

    async fn find_members(&self, talk_id: &talk::Id) -> super::Result<HashSet<user::Id>> {
        let talk_key = cache::Key::Talk(talk_id);
        let members = self
            .redis
            .smembers::<HashSet<user::Id>>(talk_key.clone())
            .await;

        match members {
            Some(m) if !m.is_empty() => Ok(m),
            _ => {
                let m = self.repo.find_by_talk_id(talk_id)?;
                let m = HashSet::from_iter(m);

                self.redis.sadd(talk_key.clone(), &m).await;
                self.redis.expire(talk_key).await;

                Ok(m)
            }
        }
    }

    async fn check_member(&self, talk_id: &talk::Id, auth_user: &auth::User) -> super::Result<()> {
        let members = self.find_members(talk_id).await?;

        if !members.contains(auth_user.id()) {
            return Err(super::Error::NotMember);
        }

        Ok(())
    }

    async fn find_name(&self, id: &user::Id) -> super::Result<String> {
        if let Some(name) = self.find_cached_name(id).await {
            Ok(name)
        } else {
            let u = self.find_one(id).await?;
            Ok(u.name().to_owned())
        }
    }

    async fn find_picture(&self, id: &user::Id) -> super::Result<Picture> {
        if let Some(p) = self.find_cached_picture(id).await {
            Picture::try_from(p.as_str())
        } else {
            let u = self.find_one(id).await?;
            Ok(u.picture().clone())
        }
    }

    fn exists(&self, id: &user::Id) -> super::Result<bool> {
        self.repo.exists(id)
    }

    fn search(&self, nickname: &Nickname, auth_user: &auth::User) -> super::Result<Vec<UserDto>> {
        let users = self
            .repo
            .find_by_nickname_like_and_excluding(nickname, auth_user.nickname())?;

        Ok(users.into_iter().map(UserDto::from).collect())
    }

    async fn notify_online(&self, id: &user::Id) {
        self.notify_online_status_change(id, true).await;
    }

    async fn notify_offline(&self, id: &user::Id) {
        self.notify_online_status_change(id, false).await;
    }
}

// notifications
impl UserServiceImpl {
    async fn notify_online_status_change(&self, id: &user::Id, online: bool) {
        match self
            .contact_service
            .find_by_user_id_and_status(id, &contact::Status::Accepted)
        {
            Ok(contacts) => {
                let subjects = contacts
                    .iter()
                    .map(|c| event::Subject::Notifications(c.recipient()))
                    .collect::<Vec<_>>();

                let status = OnlineStatus::new(id.to_owned(), online);
                self.event_service
                    .broadcast(
                        &subjects,
                        event::Notification::OnlineStatusChange(status.clone()).into(),
                    )
                    .await;
            }
            Err(e) => {
                error!("failed to find contacts for sub: {e:?}");
            }
        }
    }
}

// cache operations
impl UserServiceImpl {
    async fn cache(&self, u: &UserDto) {
        let k = cache::Key::User(u.id());
        self.redis.json_set_ex(k, u).await;
    }

    async fn find_cached(&self, id: &user::Id) -> Option<UserDto> {
        let k = cache::Key::User(id);
        self.redis.json_get::<UserDto>(k, None).await
    }

    async fn find_cached_name(&self, id: &user::Id) -> Option<String> {
        self.find_cached_field(id, ".name").await
    }

    async fn find_cached_picture(&self, id: &user::Id) -> Option<String> {
        self.find_cached_field(id, ".picture").await
    }

    async fn find_cached_field(&self, id: &user::Id, path: &str) -> Option<String> {
        let k = cache::Key::User(id);
        self.redis
            .json_get::<String>(k, Some(path))
            .await
            .map(|r| r.replace('\"', "")) // normalize json string
    }
}
