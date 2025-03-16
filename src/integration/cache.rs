use std::env;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use log::{error, warn};
use redis::{AsyncCommands, JsonAsyncCommands};
use serde::Serialize;

use crate::user::model::UserInfo;
use crate::{talk, user};

#[derive(Clone)]
pub struct Redis {
    con: redis::aio::ConnectionManager,
}

impl Redis {
    pub async fn set<V>(&self, key: Key, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let mut con = self.con.clone();
        if let Err(e) = con.set::<&Key, V, ()>(&key, value).await {
            error!("Failed to set for key {key}. Reason: {e:?}")
        }
    }

    pub async fn set_ex<V>(&self, key: Key, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let mut con = self.con.clone();
        if let Err(e) = con.set_ex::<&Key, V, ()>(&key, value, key.ttl()).await {
            error!("Failed to set_ex for key {key}. Reason: {e:?}")
        }
    }

    pub async fn json_set_ex<V>(&self, key: Key, value: V)
    where
        V: Send + Sync + Serialize,
    {
        let mut con = self.con.clone();
        if let Err(e) = con.json_set::<&Key, &str, V, ()>(&key, "$", &value).await {
            error!("Failed to json_set for key {key}. Reason: {e:?}")
        }

        self.expire(key).await
    }

    pub async fn sadd<V>(&self, key: Key, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let mut con = self.con.clone();
        if let Err(e) = con.sadd::<&Key, V, ()>(&key, value).await {
            error!("Failed to sadd for key {key}. Reason: {e:?}")
        }
    }

    pub async fn srem<V>(&self, key: Key, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let mut con = self.con.clone();
        if let Err(e) = con.srem::<&Key, V, ()>(&key, value).await {
            error!("Failed to srem for key {key}. Reason: {e:?}")
        }
    }

    pub async fn get<V>(&self, key: Key) -> Option<V>
    where
        V: redis::FromRedisValue,
    {
        let mut con = self.con.clone();
        match con.get::<&Key, V>(&key).await {
            Ok(value) => Some(value),
            Err(e) => {
                error!("Failed to get key {key}. Reason: {e:?}");
                None
            }
        }
    }

    pub async fn json_get<V>(&self, key: Key) -> Option<V>
    where
        V: redis::FromRedisValue + Clone,
    {
        let mut con = self.con.clone();
        match con.json_get::<&Key, &str, Vec<V>>(&key, ".").await {
            Ok(result) => result.first().cloned(),
            Err(e) => {
                error!("Failed to json_get key {key}. Reason: {e:?}");
                None
            }
        }
    }

    pub async fn get_del<V>(&self, key: Key) -> Option<V>
    where
        V: redis::FromRedisValue,
    {
        let mut con = self.con.clone();

        match con.get_del::<&Key, Option<V>>(&key).await {
            Ok(value) => value,
            Err(e) => {
                error!("Failed to get_del key {key}. Reason: {e:?}");
                None
            }
        }
    }

    pub async fn smembers<V>(&self, key: Key) -> Option<V>
    where
        V: redis::FromRedisValue + IntoIterator,
        V::Item: redis::FromRedisValue + PartialEq + Eq,
    {
        let mut con = self.con.clone();
        match con.smembers::<&Key, Option<V>>(&key).await {
            Ok(members) => members,
            Err(e) => {
                error!("Failed to smembers for key {key}. Reason: {e:?}");
                None
            }
        }
    }

    #[allow(dead_code)]
    pub async fn del(&self, key: Key) {
        let mut con = self.con.clone();
        if let Err(e) = con.del::<&Key, ()>(&key).await {
            error!("Failed to del key {key}. Reason: {e:?}")
        }
    }

    pub async fn expire_after(&self, key: Key, seconds: u64) {
        let mut con = self.con.clone();
        if let Err(e) = con.expire::<&Key, ()>(&key, seconds as i64).await {
            error!("Failed to expire key {key}. Reason: {e:?}")
        }
    }

    pub async fn expire(&self, key: Key) {
        let mut con = self.con.clone();
        if let Err(e) = con.expire::<&Key, ()>(&key, key.ttl() as i64).await {
            error!("Failed to expire key {key}. Reason: {e:?}")
        }
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
    pub fn env() -> Option<Self> {
        let host = env::var("REDIS_HOST").ok();
        let port = env::var("REDIS_PORT")
            .unwrap_or("6379".to_string())
            .parse()
            .ok();

        match (host, port) {
            (Some(host), Some(port)) => Some(Self { host, port }),
            _ => {
                warn!("REDIS env is not configured");
                None
            }
        }
    }

    pub async fn connect(&self) -> Redis {
        let client = match redis::Client::open(format!("redis://{}:{}", self.host, self.port)) {
            Ok(client) => client,
            Err(e) => panic!("Failed to connect to Redis: {e:?}"),
        };
        let con = match client.get_connection_manager().await {
            Ok(con) => con,
            Err(e) => panic!("Failed create Redis connection manager: {}", e),
        };

        Redis { con }
    }
}

#[derive(Clone)]
pub enum Key {
    UserInfo(user::Sub),
    Friends(user::Sub),
    Talk(talk::Id),
    Session(uuid::Uuid),
    Csrf(Arc<str>),
}

impl Key {
    /// Returns a time-to-live value in seconds for the key.
    pub fn ttl(&self) -> u64 {
        match self {
            Key::UserInfo(_) => 3600,
            Key::Friends(_) => u64::MAX,
            Key::Talk(_) => 3600,
            // Just in case if token response does not provide an expiration claim
            // fallback with this value
            Key::Session(_) => 3600,
            // Since most of IDPs don't provide a code exchange TTL through
            // introspection endpoint - we set a limit of 120 seconds.
            Key::Csrf(_) => 120,
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::UserInfo(sub) => write!(f, "userinfo:{sub}"),
            Key::Friends(sub) => write!(f, "friends:{sub}"),
            Key::Talk(id) => write!(f, "talk:{id}"),
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

impl redis::FromRedisValue for user::Sub {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<user::Sub> {
        let s = String::from_redis_value(v)?;
        Ok(user::Sub(s.into()))
    }
}

impl redis::ToRedisArgs for user::Sub {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.as_str().write_redis_args(out);
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
