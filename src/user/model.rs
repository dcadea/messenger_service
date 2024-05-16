use crate::auth::model::UserInfo;
use serde::{Deserialize, Serialize};

pub type UserId = mongodb::bson::oid::ObjectId;

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(skip)]
    _id: Option<UserId>,
    pub sub: String,
    pub nickname: String,
    pub name: String,
    pub picture: String,
    pub email: String,
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
