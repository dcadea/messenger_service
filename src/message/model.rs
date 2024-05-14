use bson::oid::ObjectId;
use mongodb::bson;
use serde::{Deserialize, Serialize};

pub type MessageId = ObjectId;

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    _id: Option<MessageId>,
    sender: String,
    pub recipient: String,
    text: String,
    timestamp: i64,
    seen: bool,
}

impl Message {
    pub fn new(sender: &str, recipient: &str, text: &str) -> Self {
        Self {
            _id: None,
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
    pub recipient: Option<Vec<String>>,
}
