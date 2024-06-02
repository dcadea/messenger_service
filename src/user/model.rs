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

#[derive(Deserialize, Clone, Debug)]
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
