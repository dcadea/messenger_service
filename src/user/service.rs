use std::collections::HashSet;
use std::sync::Arc;

use log::error;

use crate::event;
use crate::event::service::EventService;
use crate::integration::cache;
use crate::user::model::{User, UserInfo};

use super::Sub;
use super::model::OnlineStatus;
use super::repository::UserRepository;

#[derive(Clone)]
pub struct UserService {
    repo: Arc<UserRepository>,
    event_service: Arc<EventService>,
    redis: cache::Redis,
}

impl UserService {
    pub fn new(repo: UserRepository, event_service: EventService, redis: cache::Redis) -> Self {
        Self {
            repo: Arc::new(repo),
            event_service: Arc::new(event_service),
            redis,
        }
    }
}

impl UserService {
    pub async fn create(&self, user: &User) -> super::Result<()> {
        self.repo.insert(user).await
    }

    pub async fn find_user_info(&self, sub: &Sub) -> super::Result<UserInfo> {
        let cached = self.find_cached_user_info(sub).await;

        match cached {
            Some(user_info) => Ok(user_info),
            None => {
                let user_info = self.repo.find_by_sub(sub).await?.into();
                self.cache_user_info(&user_info).await;
                Ok(user_info)
            }
        }
    }

    pub async fn search_user_info(
        &self,
        nickname: &str,
        logged_nickname: &str,
    ) -> super::Result<Vec<UserInfo>> {
        let users = self
            .repo
            .search_by_nickname(nickname, logged_nickname)
            .await?;
        Ok(users.into_iter().map(|user| user.into()).collect())
    }

    pub async fn find_friends(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
        let friends = self
            .redis
            .smembers::<HashSet<Sub>>(cache::Key::Friends(sub.to_owned()))
            .await;

        match friends {
            Some(friends) => Ok(friends),
            None => self.cache_friends(sub).await,
        }
    }

    pub async fn create_friendship(&self, subs: &[Sub; 2]) -> super::Result<()> {
        let me = &subs[0];
        let you = &subs[1];
        assert_ne!(me, you);

        tokio::try_join!(
            self.repo.add_friend(me, you),
            self.repo.add_friend(you, me),
            self.cache_friends(me),
            self.cache_friends(you)
        )?;

        Ok(())
    }

    pub async fn delete_friendship(&self, subs: &[Sub; 2]) -> super::Result<()> {
        let me = &subs[0];
        let you = &subs[1];
        assert_ne!(me, you);

        tokio::try_join!(
            self.repo.remove_friend(me, you),
            self.repo.remove_friend(you, me),
        )?;

        tokio::join!(
            self.redis
                .srem(cache::Key::Friends(me.to_owned()), you.to_owned()),
            self.redis
                .srem(cache::Key::Friends(you.to_owned()), me.to_owned()),
        );

        Ok(())
    }
}

// notifications
impl UserService {
    pub async fn notify_online(&self, sub: &Sub) {
        self.notify_online_status_change(sub, true).await;
    }

    pub async fn notify_offline(&self, sub: &Sub) {
        self.notify_online_status_change(sub, false).await;
    }

    async fn notify_online_status_change(&self, sub: &Sub, online: bool) {
        match self.repo.find_friends_by_sub(sub).await {
            Ok(friend_subs) => {
                let status = OnlineStatus::new(sub.to_owned(), online);

                for fsub in friend_subs {
                    self.event_service
                        .publish(
                            &event::Subject::Notifications(&fsub),
                            &event::Notification::OnlineStatusChange(status.clone()),
                        )
                        .await;
                }
            }
            Err(e) => {
                error!("failed to find friends by sub: {e:?}");
            }
        }
    }
}

// cache operations
impl UserService {
    async fn cache_friends(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
        let friends = self.repo.find_friends_by_sub(sub).await?;

        if friends.is_empty() {
            return Ok(HashSet::with_capacity(0));
        }

        let _: () = self
            .redis
            .sadd(cache::Key::Friends(sub.to_owned()), &friends)
            .await;

        Ok(HashSet::from_iter(friends.iter().cloned()))
    }

    async fn cache_user_info(&self, user_info: &UserInfo) {
        let user_info_key = cache::Key::UserInfo(user_info.sub.to_owned());
        self.redis.json_set_ex(user_info_key, user_info).await
    }

    async fn find_cached_user_info(&self, sub: &Sub) -> Option<UserInfo> {
        let user_info_key = cache::Key::UserInfo(sub.to_owned());
        self.redis.json_get::<UserInfo>(user_info_key).await
    }
}
