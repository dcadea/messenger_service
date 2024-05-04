use mongodb::bson;
use openid::Userinfo;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize)]
pub(super) struct User {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    pub nickname: String,
    pub name: String,
    pub picture: Option<Url>,
    pub email: String,
}

impl From<Userinfo> for User {
    fn from(userinfo: Userinfo) -> Self {
        Self {
            _id: None,
            nickname: userinfo.nickname.unwrap_or_default(),
            name: userinfo.name.unwrap_or_default(),
            picture: userinfo.picture,
            email: userinfo.email.unwrap_or_default(),
        }
    }
}
