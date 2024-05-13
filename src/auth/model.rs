use crate::user::model::User;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub sub: String,
    pub nickname: String,
    pub name: String,
    pub picture: String,
    pub email: String,
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

#[derive(Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: String,
}
