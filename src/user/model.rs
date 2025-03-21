use std::collections::HashSet;

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
    contacts: HashSet<Sub>,
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

#[derive(Deserialize)]
pub struct Contacts {
    pub contacts: Vec<Sub>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub sub: Sub,
    pub nickname: String,
    pub name: String,
    pub picture: String,
    email: String,
    #[serde(skip)]
    pub contacts: HashSet<Sub>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            sub: user.sub,
            nickname: user.nickname,
            name: user.name,
            picture: user.picture,
            email: user.email,
            contacts: user.contacts,
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
            contacts: HashSet::new(),
        }
    }
}
