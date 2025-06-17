use async_trait::async_trait;
use log::error;

use crate::integration::cache;
use crate::user::model::UserInfo;
use crate::{auth, contact, event, user};

use super::model::{NewUser, OnlineStatus};
use super::{Nickname, Picture, Repository, Sub};

#[async_trait]
pub trait UserService {
    fn project(&self, user_info: &UserInfo) -> super::Result<bool>;

    async fn find_one(&self, id: &user::Id) -> super::Result<UserInfo>;

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<UserInfo>;

    async fn find_name(&self, id: &user::Id) -> super::Result<String>;

    async fn find_picture(&self, id: &user::Id) -> super::Result<Picture>;

    fn exists(&self, id: &user::Id) -> super::Result<bool>;

    fn search(&self, nickname: &Nickname, auth_user: &auth::User) -> super::Result<Vec<UserInfo>>;

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
    fn project(&self, u: &UserInfo) -> super::Result<bool> {
        self.repo.create(&NewUser::from(u)).map(|_| true)
    }

    async fn find_one(&self, id: &user::Id) -> super::Result<UserInfo> {
        if let Some(u) = self.find_cached(id).await {
            Ok(u)
        } else {
            let u = self.repo.find_by_id(id).map(UserInfo::from)?;
            self.cache(&u).await;
            Ok(u)
        }
    }

    async fn find_by_sub(&self, sub: &Sub) -> super::Result<UserInfo> {
        let u = self.repo.find_by_sub(sub).map(UserInfo::from)?;
        self.cache(&u).await;
        Ok(u)
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

    fn search(&self, nickname: &Nickname, auth_user: &auth::User) -> super::Result<Vec<UserInfo>> {
        let users = self
            .repo
            .find_by_nickname_like_and_excluding(nickname, auth_user.nickname())?;

        Ok(users.into_iter().map(UserInfo::from).collect())
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
            .await
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
    async fn cache(&self, u: &UserInfo) {
        let k = cache::Key::UserInfo(u.id());
        self.redis.json_set_ex(k, u).await;
    }

    async fn find_cached(&self, id: &user::Id) -> Option<UserInfo> {
        let k = cache::Key::UserInfo(id);
        self.redis.json_get::<UserInfo>(k, None).await
    }

    async fn find_cached_name(&self, id: &user::Id) -> Option<String> {
        self.find_cached_field(id, ".name").await
    }

    async fn find_cached_picture(&self, id: &user::Id) -> Option<String> {
        self.find_cached_field(id, ".picture").await
    }

    async fn find_cached_field(&self, id: &user::Id, path: &str) -> Option<String> {
        let k = cache::Key::UserInfo(id);
        self.redis
            .json_get::<String>(k, Some(path))
            .await
            .map(|r| r.replace('\"', "")) // normalize json string
    }
}
