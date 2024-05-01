use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    username: String,
    recipient: String,
    last_message: String,
}

#[derive(Deserialize)]
pub struct ChatParams {
    pub username: Option<String>,
}
