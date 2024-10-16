use std::env;
use std::fmt::{Display, Formatter};

use crate::user::model::UserInfo;
use crate::{chat, user};

#[derive(Clone)]
pub struct Config {
    host: String,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 6379,
        }
    }
}

impl Config {
    pub fn env() -> super::Result<Self> {
        let host = env::var("REDIS_HOST")?;
        let port = env::var("REDIS_PORT")?.parse()?;
        Ok(Self { host, port })
    }
}

pub async fn init_client(config: &Config) -> super::Result<redis::Client> {
    redis::Client::open(format!("redis://{}:{}", &config.host, &config.port))
        .map_err(super::Error::from)
}

pub async fn init(config: &Config) -> super::Result<redis::aio::ConnectionManager> {
    init_client(config)
        .await?
        .get_connection_manager()
        .await
        .map_err(super::Error::from)
}

#[derive(Clone)]
pub enum Key {
    UserInfo(user::Sub),
    UsersOnline,
    Friends(user::Sub),
    Chat(chat::Id),
    Session(String),
    Csrf(String),
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::UserInfo(sub) => write!(f, "userinfo:{sub}"),
            Key::UsersOnline => write!(f, "users:online"),
            Key::Friends(sub) => write!(f, "friends:{sub}"),
            Key::Chat(id) => write!(f, "chat:{id}"),
            Key::Session(id) => write!(f, "session:{id}"),
            Key::Csrf(csrf) => write!(f, "csrf:{csrf}"),
        }
    }
}

impl redis::ToRedisArgs for Key {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_string().write_redis_args(out);
    }
}

#[derive(Clone)]
pub struct Keyspace {
    pub key: Key,
}

impl Keyspace {
    pub fn new(key: Key) -> Self {
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

impl redis::FromRedisValue for user::Sub {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<user::Sub> {
        let s = String::from_redis_value(v)?;
        Ok(user::Sub(s))
    }
}

impl redis::ToRedisArgs for user::Sub {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.0.write_redis_args(out);
    }
}

impl redis::FromRedisValue for UserInfo {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let user_info: UserInfo = serde_json::from_str(&String::from_redis_value(value)?)?;
        Ok(user_info)
    }
}

impl redis::ToRedisArgs for UserInfo {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        serde_json::json!(self).to_string().write_redis_args(out);
    }
}
