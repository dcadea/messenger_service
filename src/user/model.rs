use serde::{Deserialize, Serialize};

use super::{Email, Id, Nickname, Sub};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    sub: Sub,
    nickname: Nickname,
    name: String,
    picture: String,
    email: Email,
}

#[cfg(test)]
impl User {
    pub fn new(
        id: Id,
        sub: Sub,
        nickname: Nickname,
        name: impl Into<String>,
        picture: impl Into<String>,
        email: Email,
    ) -> Self {
        Self {
            id: Some(id),
            sub,
            nickname,
            name: name.into(),
            picture: picture.into(),
            email,
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
    pub const fn new(sub: Sub, online: bool) -> Self {
        Self { sub, online }
    }

    pub fn id(&self) -> &str {
        self.sub.id()
    }

    pub const fn online(&self) -> bool {
        self.online
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    sub: Sub,
    nickname: Nickname,
    name: String,
    picture: String,
    email: Email,
}

impl UserInfo {
    pub const fn sub(&self) -> &Sub {
        &self.sub
    }

    pub const fn nickname(&self) -> &Nickname {
        &self.nickname
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn email(&self) -> &Email {
        &self.email
    }

    pub fn picture(&self) -> &str {
        &self.picture
    }
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
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

#[cfg(test)]
impl UserInfo {
    pub fn new(
        sub: Sub,
        nickname: Nickname,
        name: impl Into<String>,
        picture: impl Into<String>,
        email: Email,
    ) -> Self {
        Self {
            sub,
            nickname,
            name: name.into(),
            picture: picture.into(),
            email,
        }
    }
}
