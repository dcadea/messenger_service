use serde::{Deserialize, Serialize};

use super::{Id, Sub};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    sub: Sub,
    nickname: String,
    name: String,
    picture: String,
    email: String,
}

#[cfg(test)]
impl User {
    pub fn new(
        id: Id,
        sub: Sub,
        nickname: impl Into<String>,
        name: impl Into<String>,
        picture: impl Into<String>,
        email: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(id),
            sub,
            nickname: nickname.into(),
            name: name.into(),
            picture: picture.into(),
            email: email.into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OnlineStatus {
    // TODO: add lifetime
    sub: Sub,
    online: bool,
}

impl OnlineStatus {
    pub fn new(sub: Sub, online: bool) -> Self {
        Self { sub, online }
    }

    pub fn id(&self) -> &str {
        self.sub.id()
    }

    pub fn online(&self) -> bool {
        self.online
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    sub: Sub,
    nickname: String,
    name: String,
    picture: String,
    email: String,
}

impl UserInfo {
    pub fn sub(&self) -> &Sub {
        &self.sub
    }

    pub fn nickname(&self) -> &str {
        &self.nickname
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn picture(&self) -> &str {
        &self.picture
    }
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
