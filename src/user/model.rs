use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    pub sub: String,
    pub nickname: String,
    pub name: String,
    pub picture: String,
    pub email: String,
}
