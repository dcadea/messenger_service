use serde::{Deserialize, Serialize};

type UserId = mongodb::bson::oid::ObjectId;
pub type UserSub = String;

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(skip)]
    _id: Option<UserId>,
    sub: UserSub,
    nickname: String,
    name: String,
    picture: String,
    email: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub sub: UserSub,
    nickname: String,
    name: String,
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
    pub sub: Option<UserSub>,
    pub nickname: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::user::model::{User, UserSub};
    use std::fmt::{Debug, Formatter};

    impl User {
        pub fn new(sub: UserSub, nickname: &str, name: &str, picture: &str, email: &str) -> Self {
            Self {
                _id: None,
                sub,
                nickname: nickname.to_owned(),
                name: name.to_owned(),
                picture: picture.to_owned(),
                email: email.to_owned(),
            }
        }
    }

    impl Debug for User {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("User")
                .field("sub", &self.sub)
                .field("nickname", &self.nickname)
                .field("name", &self.name)
                .field("picture", &self.picture)
                .field("email", &self.email)
                .finish()
        }
    }

    impl PartialEq<Self> for User {
        fn eq(&self, other: &Self) -> bool {
            self.sub == other.sub
                && self.nickname == other.nickname
                && self.name == other.name
                && self.picture == other.picture
                && self.email == other.email
        }
    }
}
