use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct Chat {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    nickname: String,
    recipient: String,
    last_message: String,
}

#[derive(Deserialize)]
pub(super) struct ChatParams {
    pub nickname: Option<String>,
}
