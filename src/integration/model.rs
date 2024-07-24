use std::fmt::{Display, Formatter};

use crate::{chat::model::ChatId, user::model::Sub};

#[derive(Clone)]
pub enum CacheKey {
    UserInfo(Sub),
    UsersOnline,
    Friends(Sub),
    Chat(ChatId),
}

impl Display for CacheKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheKey::UserInfo(sub) => write!(f, "userinfo:{sub}"),
            CacheKey::UsersOnline => write!(f, "users:online"),
            CacheKey::Friends(sub) => write!(f, "friends:{sub}"),
            CacheKey::Chat(id) => write!(f, "chat:{id}"),
        }
    }
}

impl redis::ToRedisArgs for CacheKey {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_string().write_redis_args(out);
    }
}

#[derive(Clone)]
pub struct Keyspace {
    pub key: CacheKey,
}

impl Keyspace {
    pub fn new(key: CacheKey) -> Self {
        Self { key }
    }
}

impl Display for Keyspace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "__keyspace@0__:{}", &self.key)
    }
}

impl redis::ToRedisArgs for Keyspace {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_string().write_redis_args(out);
    }
}
