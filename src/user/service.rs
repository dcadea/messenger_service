use std::collections::HashSet;
use std::sync::Arc;

use crate::integration::{self, cache};
use crate::user::model::{User, UserInfo};

use super::repository::UserRepository;
use super::Sub;

#[derive(Clone)]
pub struct UserService {
    repository: Arc<UserRepository>,
    redis: integration::cache::Redis,
}

impl UserService {
    pub fn new(repository: UserRepository, redis: integration::cache::Redis) -> Self {
        Self {
            repository: Arc::new(repository),
            redis,
        }
    }
}

impl UserService {
    pub async fn create(&self, user: &User) -> super::Result<()> {
        self.repository.insert(user).await
    }

    pub async fn find_user_info(&self, sub: &Sub) -> super::Result<UserInfo> {
        let cached_user_info = self.find_cached_user_info(sub).await;

        match cached_user_info {
            Some(user_info) => Ok(user_info),
            None => {
                let user_info = self.repository.find_by_sub(sub).await?.into();
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
            .repository
            .search_by_nickname(nickname, logged_nickname)
            .await?;
        Ok(users.into_iter().map(|user| user.into()).collect())
    }

    pub async fn create_friendship(&self, subs: &[Sub; 2]) -> super::Result<()> {
        assert_ne!(&subs[0], &subs[1]);

        tokio::try_join!(
            self.repository.add_friend(&subs[0], &subs[1]),
            self.repository.add_friend(&subs[1], &subs[0]),
        )?;

        Ok(())
    }

    pub async fn delete_friendship(&self, subs: &[Sub; 2]) -> super::Result<()> {
        assert_ne!(&subs[0], &subs[1]);

        tokio::try_join!(
            self.repository.remove_friend(&subs[0], &subs[1]),
            self.repository.remove_friend(&subs[1], &subs[0]),
        )?;

        self.invalidate_friends(subs).await;

        Ok(())
    }
}

// cache operations
impl UserService {
    // TODO: online users feature
    // pub async fn add_online_user(&self, sub: &Sub) {
    //     debug!("Adding to online users: {:?}", sub);
    //     self.redis.sadd(cache::Key::UsersOnline, sub).await
    // }

    // pub async fn get_online_friends(&self, sub: &Sub) -> Option<HashSet<Sub>> {
    //     self.redis
    //         .sinter::<Sub>(vec![
    //             cache::Key::UsersOnline,
    //             cache::Key::Friends(sub.to_owned()),
    //         ])
    //         .await
    // }

    // pub async fn remove_online_user(&self, sub: &Sub) {
    //     debug!("Removing from online users: {:?}", sub);
    //     self.redis.srem(cache::Key::UsersOnline, sub).await
    // }

    pub async fn cache_friends(&self, sub: &Sub) -> super::Result<()> {
        let friends = self.repository.find_friends_by_sub(sub).await?;

        if friends.is_empty() {
            return Ok(());
        }

        let _: () = self
            .redis
            .sadd(cache::Key::Friends(sub.to_owned()), friends)
            .await;

        Ok(())
    }

    pub async fn find_cached_friends(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
        let friends = self
            .redis
            .smembers::<HashSet<Sub>>(cache::Key::Friends(sub.to_owned()))
            .await;

        match friends {
            Some(friends) => Ok(friends),
            None => Err(super::Error::NoFriends(sub.to_owned())),
        }
    }

    async fn invalidate_friends(&self, subs: &[Sub]) {
        for sub in subs {
            let _: () = self.redis.del(cache::Key::Friends(sub.to_owned())).await;
        }
    }

    async fn cache_user_info(&self, user_info: &UserInfo) {
        let cache_key = cache::Key::UserInfo(user_info.sub.to_owned());
        self.redis.json_set_ex(cache_key, user_info).await
    }

    async fn find_cached_user_info(&self, sub: &Sub) -> Option<UserInfo> {
        let sub = cache::Key::UserInfo(sub.to_owned());
        self.redis.json_get::<UserInfo>(sub).await
    }
}
