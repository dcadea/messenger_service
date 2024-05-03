use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct User {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    pub nickname: String,
    pub name: String,
    pub picture: String,
    pub email: String,
}

#[derive(Deserialize)]
pub(super) struct CallbackParams {
    pub code: String,
}
