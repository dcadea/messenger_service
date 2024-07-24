use std::fmt::Display;

use serde::{Deserialize, Serialize};

type UserId = mongodb::bson::oid::ObjectId;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Sub(String);

impl Display for Sub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Sub {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Sub {
    fn deserialize<D>(deserializer: D) -> Result<Sub, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Sub(s))
    }
}

impl From<Sub> for mongodb::bson::Bson {
    fn from(val: Sub) -> Self {
        mongodb::bson::Bson::String(val.0)
    }
}

impl redis::FromRedisValue for Sub {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Sub> {
        let s = String::from_redis_value(v)?;
        Ok(Sub(s))
    }
}

impl redis::ToRedisArgs for Sub {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.0.write_redis_args(out);
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(skip)]
    _id: Option<UserId>,
    sub: Sub,
    nickname: String,
    name: String,
    picture: String,
    email: String,
    friends: Vec<Sub>, // vec of sub
}

#[derive(Deserialize)]
pub struct Friends {
    pub friends: Vec<Sub>, // vec of sub
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub sub: Sub,
    nickname: String,
    pub name: String,
    picture: String,
    email: String,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            sub: user.sub,
            nickname: user.nickname,
            name: user.name,
            picture: user.picture,
            email: user.email,
        }
    }
}

impl From<UserInfo> for User {
    fn from(info: UserInfo) -> Self {
        Self {
            _id: None,
            sub: info.sub,
            nickname: info.nickname,
            name: info.name,
            picture: info.picture,
            email: info.email,
            friends: vec![],
        }
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

#[derive(Deserialize)]
pub(super) struct UserParams {
    pub sub: Option<Sub>,
    pub nickname: Option<String>,
}
