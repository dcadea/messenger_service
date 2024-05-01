use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct Chat {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    username: String,
    recipient: String,
    last_message: String,
}

#[derive(Deserialize)]
pub(super) struct ChatParams {
    pub username: Option<String>,
}
