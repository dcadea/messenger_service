use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct User {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    pub username: String,
    pub password: String,
}
