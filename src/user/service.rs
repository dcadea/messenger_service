use async_trait::async_trait;
use log::error;

use crate::integration::cache;
use crate::user::model::UserInfo;
use crate::{auth, contact, event};

use super::model::OnlineStatus;
use super::{Nickname, Picture, Repository, Sub};

#[async_trait]
pub trait UserService {
    async fn project(&self, user_info: &UserInfo) -> super::Result<bool>;

    async fn find_one(&self, sub: &Sub) -> super::Result<UserInfo>;

    async fn find_name(&self, sub: &Sub) -> super::Result<String>;

    async fn find_picture(&self, sub: &Sub) -> super::Result<Picture>;

    async fn exists(&self, sub: &Sub) -> super::Result<bool>;

    async fn search(
        &self,
        nickname: &Nickname,
        auth_user: &auth::User,
    ) -> super::Result<Vec<UserInfo>>;

    async fn notify_online(&self, sub: &Sub);

    async fn notify_offline(&self, sub: &Sub);
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
    async fn project(&self, user_info: &UserInfo) -> super::Result<bool> {
        let user = user_info.to_owned().into();
        self.repo.insert(&user).await
    }

    async fn find_one(&self, sub: &Sub) -> super::Result<UserInfo> {
        let cached = self.find_cached(sub).await;

        if let Some(user_info) = cached {
            Ok(user_info)
        } else {
            let user_info = self.repo.find_by_sub(sub).await?.into();
            self.cache(&user_info).await;
            Ok(user_info)
        }
    }

    async fn find_name(&self, sub: &Sub) -> super::Result<String> {
        let cached = self.find_cached_name(sub).await;

        if let Some(name) = cached {
            Ok(name)
        } else {
            let user_info = self.find_one(sub).await?;
            Ok(user_info.name().to_owned())
        }
    }

    async fn find_picture(&self, sub: &Sub) -> super::Result<Picture> {
        let cached = self.find_cached_picture(sub).await;

        if let Some(p) = cached {
            Picture::parse(&p)
        } else {
            let user_info = self.find_one(sub).await?;
            Ok(user_info.picture().clone())
        }
    }

    async fn exists(&self, sub: &Sub) -> super::Result<bool> {
        match self.find_one(sub).await {
            Ok(_) => Ok(true),
            Err(super::Error::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn search(
        &self,
        nickname: &Nickname,
        auth_user: &auth::User,
    ) -> super::Result<Vec<UserInfo>> {
        let users = self
            .repo
            .search_by_nickname_excluding(nickname, auth_user.nickname())
            .await?;

        Ok(users.into_iter().map(Into::into).collect())
    }

    async fn notify_online(&self, sub: &Sub) {
        self.notify_online_status_change(sub, true).await;
    }

    async fn notify_offline(&self, sub: &Sub) {
        self.notify_online_status_change(sub, false).await;
    }
}

// notifications
impl UserServiceImpl {
    async fn notify_online_status_change(&self, sub: &Sub, online: bool) {
        match self
            .contact_service
            .find_by_sub_and_status(sub, &contact::Status::Accepted)
            .await
        {
            Ok(contacts) => {
                let subjects = contacts
                    .iter()
                    .map(|c| event::Subject::Notifications(c.recipient()))
                    .collect::<Vec<_>>();

                let status = OnlineStatus::new(sub.to_owned(), online);
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
    async fn cache(&self, user_info: &UserInfo) {
        let user_info_key = cache::Key::UserInfo(user_info.sub());
        self.redis.json_set_ex(user_info_key, user_info).await;
    }

    async fn find_cached(&self, sub: &Sub) -> Option<UserInfo> {
        let user_info_key = cache::Key::UserInfo(sub);
        self.redis.json_get::<UserInfo>(user_info_key, None).await
    }

    async fn find_cached_name(&self, sub: &Sub) -> Option<String> {
        let user_info_key = cache::Key::UserInfo(sub);
        let result = self
            .redis
            .json_get::<String>(user_info_key, Some(".name"))
            .await;

        // normalize json string
        result.map(|r| r.replace('\"', ""))
    }

    async fn find_cached_picture(&self, sub: &Sub) -> Option<String> {
        let user_info_key = cache::Key::UserInfo(sub);
        let result = self
            .redis
            .json_get::<String>(user_info_key, Some(".picture"))
            .await;

        // normalize json string
        result.map(|r| r.replace('\"', ""))
    }
}
