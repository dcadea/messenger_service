use std::collections::HashSet;
use std::sync::Arc;

use redis::AsyncCommands;

use crate::integration::cache;
use crate::user::model::{User, UserInfo};

use super::repository::UserRepository;
use super::Sub;

// TODO: use ttl from token response
const USER_INFO_TTL: u64 = 3600;

#[derive(Clone)]
pub struct UserService {
    repository: Arc<UserRepository>,
    redis_con: redis::aio::ConnectionManager,
}

impl UserService {
    pub fn new(repository: UserRepository, redis_con: redis::aio::ConnectionManager) -> Self {
        Self {
            repository: Arc::new(repository),
            redis_con,
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
                self.cache_user_info(&user_info).await?;
                Ok(user_info)
            }
        }
    }

    pub async fn search_user_info(&self, nickname: &str) -> super::Result<Vec<UserInfo>> {
        let users = self.repository.search_by_nickname(nickname).await?;
        Ok(users.into_iter().map(|user| user.into()).collect())
    }
}

// cache operations
impl UserService {
    pub async fn add_online_user(&self, sub: &Sub) -> super::Result<()> {
        let mut con = self.redis_con.clone();
        let _: () = con.sadd(cache::Key::UsersOnline, sub).await?;
        Ok(())
    }

    pub async fn get_online_friends(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
        let mut con = self.redis_con.clone();
        let online_users: HashSet<Sub> = con
            .sinter(&[cache::Key::UsersOnline, cache::Key::Friends(sub.to_owned())])
            .await?;
        Ok(online_users)
    }

    pub async fn remove_online_user(&self, sub: &Sub) -> super::Result<()> {
        let mut con = self.redis_con.clone();
        let _: () = con.srem(cache::Key::UsersOnline, sub).await?;
        Ok(())
    }

    pub async fn cache_friends(&self, sub: &Sub) -> super::Result<()> {
        let friends = self.repository.find_friends_by_sub(sub).await?;

        let mut con = self.redis_con.clone();
        let _: () = con
            .sadd(cache::Key::Friends(sub.to_owned()), friends)
            .await?;
        Ok(())
    }

    async fn cache_user_info(&self, user_info: &UserInfo) -> super::Result<()> {
        let mut con = self.redis_con.clone();
        let cache_key = cache::Key::UserInfo(user_info.sub.to_owned());
        let _: () = con.set_ex(cache_key, user_info, USER_INFO_TTL).await?;
        Ok(())
    }

    async fn find_cached_user_info(&self, sub: &Sub) -> Option<UserInfo> {
        let mut con = self.redis_con.clone();
        let cache_key = cache::Key::UserInfo(sub.to_owned());
        let cached_user_info: Option<UserInfo> = con.get(cache_key).await.ok();
        cached_user_info
    }
}
