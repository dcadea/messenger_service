use serde::{Deserialize, Serialize};

use super::{Id, Sub};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    sub: Sub,
    nickname: String,
    name: String,
    picture: String,
    email: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OnlineStatus {
    // TODO: add lifetime
    sub: Sub,
    pub online: bool,
}

impl OnlineStatus {
    pub fn new(sub: Sub, online: bool) -> Self {
        Self { sub, online }
    }

    pub fn id(&self) -> &str {
        self.sub.id()
    }
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
            id: None,
            sub: info.sub,
            nickname: info.nickname,
            name: info.name,
            picture: info.picture,
            email: info.email,
        }
    }
}
