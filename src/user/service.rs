use redis::Commands;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::user::model::{User, UserInfo};

use super::repository::UserRepository;
use super::Result;

const USER_INFO_TTL: u64 = 3600;

#[derive(Clone)]
pub struct UserService {
    redis_con: Arc<RwLock<redis::Connection>>,
    repository: Arc<UserRepository>,
}

impl UserService {
    pub fn new(redis_con: RwLock<redis::Connection>, repository: UserRepository) -> Self {
        Self {
            redis_con: Arc::new(redis_con),
            repository: Arc::new(repository),
        }
    }
}

impl UserService {
    pub async fn create(&self, user: &User) -> Result<()> {
        self.repository.insert(user).await
    }

    pub async fn find_user_info(&self, sub: &str) -> Result<UserInfo> {
        let mut con = self.redis_con.write().await;
        let key = format!("userinfo:{}", sub);

        let cached: Option<String> = con.get(&key).ok();

        match cached {
            Some(value) => {
                let user_info: UserInfo = serde_json::from_str(&value)?;
                Ok(user_info)
            }
            None => {
                let user_info = self.repository.find_by_sub(sub).await?.into();
                let _: () = con.set_ex(&key, json!(user_info).to_string(), USER_INFO_TTL)?;
                Ok(user_info)
            }
        }
    }
}
