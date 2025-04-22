use log::error;

use crate::integration::cache;
use crate::user::model::{User, UserInfo};
use crate::{auth, contact, event};

use super::model::OnlineStatus;
use super::{Repository, Sub};

#[async_trait::async_trait]
pub trait UserService {
    async fn create(&self, user: &User) -> super::Result<()>;

    async fn find_user_info(&self, sub: &Sub) -> super::Result<UserInfo>;

    async fn search_user_info(
        &self,
        nickname: &str,
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

#[async_trait::async_trait]
impl UserService for UserServiceImpl {
    async fn create(&self, user: &User) -> super::Result<()> {
        self.repo.insert(user).await
    }

    async fn find_user_info(&self, sub: &Sub) -> super::Result<UserInfo> {
        let cached = self.find_cached_user_info(sub).await;

        if let Some(user_info) = cached {
            Ok(user_info)
        } else {
            let user_info = self.repo.find_by_sub(sub).await?.into();
            self.cache_user_info(&user_info).await;
            Ok(user_info)
        }
    }

    async fn search_user_info(
        &self,
        nickname: &str,
        auth_user: &auth::User,
    ) -> super::Result<Vec<UserInfo>> {
        let users = self
            .repo
            .search_by_nickname_excluding(nickname, &auth_user.nickname)
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
                let status = OnlineStatus::new(sub.to_owned(), online);

                for c in contacts {
                    self.event_service
                        .publish(
                            &event::Subject::Notifications(&c.recipient),
                            event::Notification::OnlineStatusChange(status.clone()).into(),
                        )
                        .await;
                }
            }
            Err(e) => {
                error!("failed to find contacts for sub: {e:?}");
            }
        }
    }
}

// cache operations
impl UserServiceImpl {
    async fn cache_user_info(&self, user_info: &UserInfo) {
        let user_info_key = cache::Key::UserInfo(user_info.sub.clone());
        self.redis.json_set_ex(user_info_key, user_info).await;
    }

    async fn find_cached_user_info(&self, sub: &Sub) -> Option<UserInfo> {
        let user_info_key = cache::Key::UserInfo(sub.clone());
        self.redis.json_get::<UserInfo>(user_info_key).await
    }
}
