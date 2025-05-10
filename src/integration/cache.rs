use std::env;
use std::fmt::{Display, Formatter};
use std::time::Duration;

use log::{error, trace, warn};
use messenger_service::Raw;
use redis::{AsyncCommands, JsonAsyncCommands};
use serde::Serialize;

use crate::user::model::UserInfo;
use crate::{auth, talk, user};

#[derive(Clone)]
pub struct Redis {
    con: redis::aio::ConnectionManager,
}

impl Redis {
    pub async fn set<V>(&self, key: Key<'_>, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        trace!("SET -> {key:?}");
        let mut con = self.con.clone();
        if let Err(e) = con.set::<_, _, ()>(&key, value).await {
            error!("Failed to SET on {key:?}. Reason: {e:?}");
        }
    }

    /// Set a value with a key based expiration time.
    pub async fn set_ex<V>(&self, key: Key<'_>, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        trace!("SET_EX -> {key:?}");
        let mut con = self.con.clone();
        if let Err(e) = con.set_ex::<_, _, ()>(&key, value, key.ttl()).await {
            error!("Failed to SET_EX on {key:?}. Reason: {e:?}");
        }
    }

    /// Set a value with an explicit expiration time.
    pub async fn set_ex_explicit<V>(&self, key: Key<'_>, value: V, ttl: &Duration)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        trace!("SET_EX -> {key:?}");
        let mut con = self.con.clone();
        if let Err(e) = con.set_ex::<_, _, ()>(&key, value, ttl.as_secs()).await {
            error!("Failed to SET_EX on {key:?}. Reason: {e:?}");
        }
    }

    pub async fn json_set_ex<V>(&self, key: Key<'_>, value: V)
    where
        V: Send + Sync + Serialize,
    {
        trace!("JSON_SET_EX -> {key:?}");
        let mut con = self.con.clone();
        if let Err(e) = con.json_set::<_, _, _, ()>(&key, "$", &value).await {
            error!("Failed to JSON_SET_EX on {key:?}. Reason: {e:?}");
        }

        self.expire(key).await;
    }

    pub async fn sadd<V>(&self, key: Key<'_>, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        trace!("SADD -> {key:?}");
        let mut con = self.con.clone();
        if let Err(e) = con.sadd::<_, _, ()>(&key, value).await {
            error!("Failed to SADD on {key:?}. Reason: {e:?}");
        }
    }

    pub async fn srem<V>(&self, key: Key<'_>, value: V)
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        trace!("SREM -> {key:?}");
        let mut con = self.con.clone();
        if let Err(e) = con.srem::<_, _, ()>(&key, value).await {
            error!("Failed to SREM on {key:?}. Reason: {e:?}");
        }
    }

    pub async fn get<V>(&self, key: Key<'_>) -> Option<V>
    where
        V: redis::FromRedisValue,
    {
        let mut con = self.con.clone();
        match con.get::<_, Option<V>>(&key).await {
            Ok(value) => {
                let status = if value.is_some() { "Hit" } else { "Miss" };
                trace!("GET ({status}) -> {key:?}");
                value
            }
            Err(e) => {
                error!("Failed to GET on {key:?}. Reason: {e:?}");
                None
            }
        }
    }

    pub async fn json_get<V>(&self, key: Key<'_>, path: Option<&str>) -> Option<V>
    where
        V: redis::FromRedisValue + Clone,
    {
        let mut con = self.con.clone();
        match con
            .json_get::<_, _, Vec<V>>(&key, path.unwrap_or("."))
            .await
        {
            Ok(result) => {
                let value = result.first().cloned();
                let status = if value.is_some() { "Hit" } else { "Miss" };
                trace!("JSON_GET ({status}) -> {key:?}");
                value
            }
            Err(e) => {
                error!("Failed to JSON_GET on {key:?}. Reason: {e:?}");
                None
            }
        }
    }

    pub async fn get_del<V>(&self, key: Key<'_>) -> Option<V>
    where
        V: redis::FromRedisValue,
    {
        let mut con = self.con.clone();
        match con.get_del::<_, Option<V>>(&key).await {
            Ok(value) => {
                let status = if value.is_some() { "Hit" } else { "Miss" };
                trace!("GETDEL ({status}) -> {key:?}");
                value
            }
            Err(e) => {
                error!("Failed to GETDEL on {key:?}. Reason: {e:?}");
                None
            }
        }
    }

    pub async fn smembers<V>(&self, key: Key<'_>) -> Option<V>
    where
        V: redis::FromRedisValue + IntoIterator,
        V::Item: redis::FromRedisValue + PartialEq,
    {
        trace!("SMEMBERS -> {key:?}");
        let mut con = self.con.clone();
        match con.smembers::<_, Option<V>>(&key).await {
            Ok(members) => members,
            Err(e) => {
                error!("Failed to SMEMBERS on {key:?}. Reason: {e:?}");
                None
            }
        }
    }

    pub async fn expire(&self, key: Key<'_>) {
        trace!("EXPIRE -> {key:?}");
        let mut con = self.con.clone();
        match i64::try_from(key.ttl()) {
            Ok(ttl) => {
                if let Err(e) = con.expire::<_, ()>(&key, ttl).await {
                    error!("Failed to EXPIRE on {key:?}. Reason: {e:?}");
                }
            }
            Err(e) => error!("Failed to cast to i64: {e:?}"),
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
            .unwrap_or_else(|_| "6379".to_string())
            .parse()
            .ok();

        if let (Some(host), Some(port)) = (host, port) {
            Some(Self { host, port })
        } else {
            warn!("REDIS env is not configured");
            None
        }
    }

    pub async fn connect(&self) -> Redis {
        let client = match redis::Client::open(format!("redis://{}:{}", self.host, self.port)) {
            Ok(client) => client,
            Err(e) => panic!("Failed to connect to Redis: {e:?}"),
        };
        let con = match client.get_connection_manager().await {
            Ok(con) => con,
            Err(e) => panic!("Failed create Redis connection manager: {e}"),
        };

        Redis { con }
    }
}

#[derive(Clone, Debug)]
pub enum Key<'a> {
    UserInfo(&'a user::Sub),
    Contacts(&'a user::Sub),
    Talk(&'a talk::Id),
    Session(&'a auth::Session),
    Csrf(&'a auth::Csrf),
}

impl Key<'_> {
    /// Returns a time-to-live value in seconds for the key.
    pub const fn ttl(&self) -> u64 {
        match self {
            // Just in case if token response does not provide an expiration claim
            // fallback with 3600 for Key::Session
            Key::UserInfo(_) | Key::Talk(_) | Key::Session(_) => 3600,
            Key::Contacts(_) => u64::MAX,
            // Since most of IDPs don't provide a code exchange TTL through
            // introspection endpoint - we set a limit of 120 seconds.
            Key::Csrf(_) => 120,
        }
    }
}

impl Display for Key<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserInfo(sub) => write!(f, "userinfo:{sub}"),
            Self::Contacts(sub) => write!(f, "contacts:{sub}"),
            Self::Talk(id) => write!(f, "talk:{id}"),
            Self::Session(s) => write!(f, "session:{}", s.raw()),
            Self::Csrf(csrf) => write!(f, "csrf:{}", csrf.raw()),
        }
    }
}

impl redis::ToRedisArgs for Key<'_> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_string().write_redis_args(out);
    }
}

impl redis::FromRedisValue for user::Sub {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        let s = String::from_redis_value(v)?;
        Ok(Self(s.into()))
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
        let user_info: Self = serde_json::from_str(&String::from_redis_value(value)?)?;
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

impl redis::FromRedisValue for auth::Csrf {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        Ok(Self::new(String::from_redis_value(v)?))
    }
}
