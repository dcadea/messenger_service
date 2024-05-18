use serde::{Deserialize, Serialize};

use crate::util::serialize_object_id;

pub type ChatId = mongodb::bson::oid::ObjectId;

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<ChatId>,
    pub sender: String,
    pub recipient: String,
    last_message: String,
}

impl Chat {
    pub fn new(sender: &str, recipient: &str, last_message: &str) -> Self {
        Self {
            id: None,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            last_message: last_message.to_string(),
        }
    }
}
