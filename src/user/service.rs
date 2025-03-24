use std::collections::HashSet;

use log::error;

use crate::event;
use crate::integration::cache;
use crate::user::model::{User, UserInfo};

use super::model::OnlineStatus;
use super::{Repository, Sub};

#[async_trait::async_trait]
pub trait UserService {
    async fn create(&self, user: &User) -> super::Result<()>;

    async fn find_user_info(&self, sub: &Sub) -> super::Result<UserInfo>;

    async fn search_user_info(
        &self,
        nickname: &str,
        logged_nickname: &str,
    ) -> super::Result<Vec<UserInfo>>;

    async fn find_contacts(&self, sub: &Sub) -> super::Result<HashSet<Sub>>;

    // TODO: revisit this
    async fn _create_contact(&self, subs: &[Sub; 2]) -> super::Result<()>;

    // TODO: revisit this
    async fn _delete_contact(&self, subs: &[Sub; 2]) -> super::Result<()>;

    async fn notify_online(&self, sub: &Sub);

    async fn notify_offline(&self, sub: &Sub);
}

#[derive(Clone)]
pub struct UserServiceImpl {
    repo: Repository,
    event_service: event::Service,
    redis: cache::Redis,
}

impl UserServiceImpl {
    pub fn new(repo: Repository, event_service: event::Service, redis: cache::Redis) -> Self {
        Self {
            repo,
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
        logged_nickname: &str,
    ) -> super::Result<Vec<UserInfo>> {
        let users = self
            .repo
            .search_by_nickname(nickname, logged_nickname)
            .await?;
        Ok(users.into_iter().map(Into::into).collect())
    }

    async fn find_contacts(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
        let contacts = self
            .redis
            .smembers::<HashSet<Sub>>(cache::Key::Contacts(sub.to_owned()))
            .await;

        match contacts {
            Some(c) => Ok(c),
            None => self.cache_contacts(sub).await,
        }
    }

    // TODO: revisit this
    async fn _create_contact(&self, subs: &[Sub; 2]) -> super::Result<()> {
        let me = &subs[0];
        let you = &subs[1];
        assert_ne!(me, you);

        tokio::try_join!(
            self.repo.add_contact(me, you),
            self.repo.add_contact(you, me),
            self.cache_contacts(me),
            self.cache_contacts(you)
        )?;

        Ok(())
    }

    // TODO: revisit this
    async fn _delete_contact(&self, subs: &[Sub; 2]) -> super::Result<()> {
        let me = &subs[0];
        let you = &subs[1];
        assert_ne!(me, you);

        self.repo.remove_contact(me, you).await?;

        tokio::join!(
            self.redis
                .srem(cache::Key::Contacts(me.to_owned()), you.to_owned()),
            self.redis
                .srem(cache::Key::Contacts(you.to_owned()), me.to_owned()),
        );

        Ok(())
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
        match self.repo.find_contacts_for_sub(sub).await {
            Ok(contact) => {
                let status = OnlineStatus::new(sub.to_owned(), online);

                for c in contact {
                    self.event_service
                        .publish(
                            &event::Subject::Notifications(&c),
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
    async fn cache_contacts(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
        let contacts = self.repo.find_contacts_for_sub(sub).await?;

        if contacts.is_empty() {
            return Ok(HashSet::with_capacity(0));
        }

        let _: () = self
            .redis
            .sadd(cache::Key::Contacts(sub.clone()), &contacts)
            .await;

        Ok(contacts.iter().cloned().collect::<HashSet<_>>())
    }

    async fn cache_user_info(&self, user_info: &UserInfo) {
        let user_info_key = cache::Key::UserInfo(user_info.sub.clone());
        self.redis.json_set_ex(user_info_key, user_info).await;
    }

    async fn find_cached_user_info(&self, sub: &Sub) -> Option<UserInfo> {
        let user_info_key = cache::Key::UserInfo(sub.clone());
        self.redis.json_get::<UserInfo>(user_info_key).await
    }
}
