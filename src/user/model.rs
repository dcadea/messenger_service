use diesel::prelude::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Email, Nickname, Picture, Sub};

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    id: Uuid,
    sub: String,
    nickname: String,
    name: String,
    picture: String,
    email: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    sub: &'a str,
    nickname: &'a str,
    name: &'a str,
    picture: &'a str,
    email: &'a str,
}

impl<'a> From<&'a UserInfo> for NewUser<'a> {
    fn from(ui: &'a UserInfo) -> Self {
        Self {
            sub: ui.sub.as_str(),
            nickname: ui.nickname.as_str(),
            name: &ui.name,
            picture: ui.picture.as_str(),
            email: ui.email.as_str(),
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
    picture: Picture,
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

    pub const fn picture(&self) -> &Picture {
        &self.picture
    }
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            sub: Sub(user.sub),
            nickname: Nickname(user.nickname),
            name: user.name,
            picture: Picture(user.picture),
            email: Email(user.email),
        }
    }
}

#[cfg(test)]
impl UserInfo {
    pub fn new(
        sub: Sub,
        nickname: Nickname,
        name: impl Into<String>,
        picture: Picture,
        email: Email,
    ) -> Self {
        Self {
            sub,
            nickname,
            name: name.into(),
            picture,
            email,
        }
    }
}
