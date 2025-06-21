use diesel::prelude::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::auth::{self, UserInfo};

use super::{Email, Id, Nickname, Picture, Sub};

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    id: Id,
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

impl<'a> From<&'a auth::UserInfo> for NewUser<'a> {
    fn from(ui: &'a auth::UserInfo) -> Self {
        Self {
            sub: ui.sub().as_str(),
            nickname: ui.nickname().as_str(),
            name: ui.name(),
            picture: ui.picture().as_str(),
            email: ui.email().as_str(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OnlineStatus {
    // TODO: add lifetime
    id: Id,
    online: bool,
}

impl OnlineStatus {
    pub const fn new(id: Id, online: bool) -> Self {
        Self { id, online }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub const fn online(&self) -> bool {
        self.online
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserDto {
    id: Id,
    sub: Sub,
    nickname: Nickname,
    name: String,
    picture: Picture,
    email: Email,
}

impl UserDto {
    pub fn new(id: Id, ui: &UserInfo) -> Self {
        Self {
            id,
            sub: ui.sub().clone(),
            nickname: ui.nickname().clone(),
            name: ui.name().to_string(),
            picture: ui.picture().clone(),
            email: ui.email().clone(),
        }
    }

    pub const fn id(&self) -> &Id {
        &self.id
    }

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

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            sub: Sub(user.sub),
            nickname: Nickname(user.nickname),
            name: user.name,
            picture: Picture(user.picture),
            email: Email(user.email),
        }
    }
}

// #[cfg(test)]
// impl UserDto {
//     pub fn new(
//         id: Id,
//         sub: Sub,
//         nickname: Nickname,
//         name: impl Into<String>,
//         picture: Picture,
//         email: Email,
//     ) -> Self {
//         Self {
//             id,
//             sub,
//             nickname,
//             name: name.into(),
//             picture,
//             email,
//         }
//     }
// }
