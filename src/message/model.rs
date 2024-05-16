use serde::{Deserialize, Serialize};

use crate::util::serialize_object_id;

pub type MessageId = mongodb::bson::oid::ObjectId;

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    id: Option<MessageId>,
    pub sender: String,
    pub recipient: String,
    text: String,
    timestamp: i64,
    seen: bool,
}

impl Message {
    pub fn new(sender: &str, recipient: &str, text: &str) -> Self {
        Self {
            id: None,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            text: text.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            seen: false,
        }
    }
}

#[derive(Deserialize)]
pub(super) struct MessageParams {
    pub companion: Option<Vec<String>>,
}
