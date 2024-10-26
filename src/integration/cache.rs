use std::collections::HashSet;
use std::env;
use std::fmt::{Display, Formatter};
use std::hash::Hash;

use anyhow::Context;
use redis::AsyncCommands;

use crate::user::model::UserInfo;
use crate::{chat, user};

#[derive(Clone)]
pub struct Redis {
    con: redis::aio::ConnectionManager,
}

impl Redis {
    pub async fn try_new(config: &Config) -> Self {
        let con = match init_client(config).get_connection_manager().await {
            Ok(con) => con,
            Err(e) => panic!("Failed create Redis connection manager: {}", e),
        };
        Self { con }
    }
}

impl Redis {
    pub async fn set_ex<V>(&self, key: Key, value: V, seconds: u64) -> anyhow::Result<()>
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let mut con = self.con.clone();
        let _: () = con.set_ex(&key, value, seconds).await.with_context(|| {
            format!("Failed to cache value for key: {key} with expiration: {seconds}")
        })?;
        Ok(())
    }

    pub async fn sadd<V>(&self, key: Key, value: V) -> anyhow::Result<()>
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let mut con = self.con.clone();
        let _: () = con
            .sadd(&key, value)
            .await
            .with_context(|| format!("Failed to cache value for key: {key}"))?;
        Ok(())
    }

    pub async fn get<V>(&self, key: Key) -> anyhow::Result<V>
    where
        V: redis::FromRedisValue,
    {
        let mut con = self.con.clone();
        let value: V = con
            .get(&key)
            .await
            .with_context(|| format!("Failed to get value from cache by key: {key}"))?;
        Ok(value)
    }

    pub async fn get_del<V>(&self, key: Key) -> anyhow::Result<V>
    where
        V: redis::FromRedisValue,
    {
        let mut con = self.con.clone();
        let value: V = con
            .get_del(&key)
            .await
            .with_context(|| format!("Failed to get and remove value from cache by key: {key}"))?;
        Ok(value)
    }

    pub async fn smembers<V>(&self, key: Key) -> anyhow::Result<Option<Vec<V>>>
    where
        V: redis::FromRedisValue + Hash + PartialEq + Eq,
    {
        let mut con = self.con.clone();
        let values: Option<Vec<V>> = con
            .smembers(&key)
            .await
            .with_context(|| format!("Failed to get values from cache by key: {key}"))?;
        Ok(values)
    }

    pub async fn sinter<V>(&self, keys: Vec<Key>) -> anyhow::Result<HashSet<V>>
    where
        V: redis::FromRedisValue + Hash + PartialEq + Eq,
    {
        let mut con = self.con.clone();
        let values: HashSet<V> = con
            .sinter(&keys)
            .await // find a way to concatenate keys into a single string
            .with_context(|| "Failed to get common values from cache by keys")?;
        Ok(values)
    }

    pub async fn del(&self, key: Key) -> anyhow::Result<()> {
        let mut con = self.con.clone();
        let _: () = con
            .del(&key)
            .await
            .with_context(|| format!("Failed to remove value frm cache by key: {key}"))?;
        Ok(())
    }

    pub async fn srem<V>(&self, key: Key, value: V) -> anyhow::Result<()>
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let mut con = self.con.clone();
        let _: () = con
            .srem(&key, value)
            .await
            .with_context(|| format!("Failed to remove value from cache by key: {key}"))?;
        Ok(())
    }

    pub async fn expire(&self, key: Key, seconds: u64) -> anyhow::Result<()> {
        let mut con = self.con.clone();
        let _: () = con
            .expire(&key, seconds as i64)
            .await
            .with_context(|| format!("Failed to set expiration for key: {key}"))?;
        Ok(())
    }
}

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
    pub fn env() -> anyhow::Result<Self> {
        let host = env::var("REDIS_HOST")?;
        let port = env::var("REDIS_PORT")?.parse()?;
        Ok(Self { host, port })
    }
}

pub fn init_client(config: &Config) -> redis::Client {
    match redis::Client::open(format!("redis://{}:{}", &config.host, &config.port)) {
        Ok(client) => client,
        Err(e) => panic!("Failed to connect to Redis: {}", e),
    }
}

#[derive(Clone)]
pub enum Key {
    UserInfo(user::Sub),
    UsersOnline,
    Friends(user::Sub),
    Chat(chat::Id),
    Session(uuid::Uuid),
    Csrf(String),
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::UserInfo(sub) => write!(f, "userinfo:{sub}"),
            Key::UsersOnline => write!(f, "users:online"),
            Key::Friends(sub) => write!(f, "friends:{sub}"),
            Key::Chat(id) => write!(f, "chat:{}", id.0),
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
