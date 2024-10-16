use serde::{Deserialize, Serialize};

use super::{Id, Sub};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(skip)]
    _id: Option<Id>,
    sub: Sub,
    nickname: String,
    name: String,
    picture: String,
    email: String,
    friends: Vec<Sub>,
}

#[derive(Deserialize)]
pub struct Friends {
    pub friends: Vec<Sub>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub sub: Sub,
    pub nickname: String,
    pub name: String,
    pub picture: String,
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
