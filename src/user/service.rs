use std::collections::HashSet;
use std::sync::Arc;

use redis::AsyncCommands;

use crate::integration::model::CacheKey;
use crate::user::model::{User, UserInfo, UserSub};

use super::repository::UserRepository;
use super::Result;

const USER_INFO_TTL: u64 = 3600;

#[derive(Clone)]
pub struct UserService {
    redis_con: redis::aio::ConnectionManager,
    repository: Arc<UserRepository>,
}

impl UserService {
    pub fn new(redis_con: redis::aio::ConnectionManager, repository: UserRepository) -> Self {
        Self {
            redis_con,
            repository: Arc::new(repository),
        }
    }
}

impl UserService {
    pub async fn create(&self, user: &User) -> Result<()> {
        self.repository.insert(user).await
    }

    pub async fn find_user_info(&self, sub: UserSub) -> Result<UserInfo> {
        let cached_user_info = self.find_cached_user_info(sub.clone()).await;

        match cached_user_info {
            Some(user_info) => Ok(user_info),
            None => {
                let user_info = self.repository.find_by_sub(&sub).await?.into();
                self.cache_user_info(&user_info).await?;
                Ok(user_info)
            }
        }
    }

    pub async fn search_user_info(&self, nickname: &str) -> Result<Vec<UserInfo>> {
        let users = self.repository.search_by_nickname(nickname).await?;
        Ok(users.into_iter().map(|user| user.into()).collect())
    }

    pub async fn add_friend(&self, sub: UserSub, friend: UserSub) -> Result<()> {
        self.repository.add_friend(&sub, &friend).await?;
        self.cache_friend(sub, friend).await?;
        Ok(())
    }
}

// cache operations
impl UserService {
    pub async fn add_online_user(&self, sub: UserSub) -> Result<()> {
        let mut con = self.redis_con.clone();
        let _: () = con.sadd(CacheKey::UsersOnline, sub).await?;
        Ok(())
    }

    pub async fn get_online_friends(&self, sub: UserSub) -> Result<HashSet<UserSub>> {
        let mut con = self.redis_con.clone();
        let online_users: HashSet<UserSub> = con
            .sinter(&[CacheKey::UsersOnline, CacheKey::Friends(sub)])
            .await?;
        Ok(online_users)
    }

    pub async fn remove_online_user(&self, sub: UserSub) -> Result<()> {
        let mut con = self.redis_con.clone();
        let _: () = con.srem(CacheKey::UsersOnline, sub).await?;
        Ok(())
    }

    pub async fn cache_friends(&self, sub: UserSub) -> Result<()> {
        let friends = self.repository.find_friends_by_sub(&sub).await?;

        let mut con = self.redis_con.clone();
        let _: () = con.sadd(CacheKey::Friends(sub), friends).await?;
        Ok(())
    }

    async fn cache_friend(&self, sub: UserSub, friend: UserSub) -> Result<()> {
        let mut con = self.redis_con.clone();
        let _: () = con.sadd(CacheKey::Friends(sub), friend).await?;
        Ok(())
    }

    async fn cache_user_info(&self, user_info: &UserInfo) -> Result<()> {
        let mut con = self.redis_con.clone();
        let cache_key = CacheKey::UserInfo(user_info.sub.clone());
        let _: () = con.set_ex(cache_key, user_info, USER_INFO_TTL).await?;
        Ok(())
    }

    async fn find_cached_user_info(&self, sub: UserSub) -> Option<UserInfo> {
        let mut con = self.redis_con.clone();
        let cache_key = CacheKey::UserInfo(sub);
        let cached_user_info: Option<UserInfo> = con.get(cache_key).await.ok();
        cached_user_info
    }
}
