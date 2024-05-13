use bson::oid::ObjectId;
use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    #[serde(skip)]
    _id: Option<ObjectId>,
    // TODO: create an id field
    sender: String,
    pub recipient: String,
    text: String,
    timestamp: i64,
}

impl Message {
    pub fn new(sender: &str, recipient: &str, text: &str) -> Self {
        Self {
            _id: None,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            text: text.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct MessageParams {
    pub recipient: Option<Vec<String>>,
}
