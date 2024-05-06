use chrono::Utc;
use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct Message {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    sender: String,
    recipient: String,
    text: String,
    timestamp: i64,
    seen: bool,
}

impl Message {
    pub fn from_request(sender: &str, request: MessageRequest) -> Self {
        Self {
            _id: None,
            sender: sender.to_string(),
            recipient: request.recipient,
            text: request.text,
            timestamp: Utc::now().timestamp(),
            seen: false,
        }
    }
}

#[derive(Deserialize, Clone)]
pub(super) struct MessageRequest {
    pub recipient: String,
    text: String,
}

#[derive(Deserialize)]
pub(super) struct MessageParams {
    pub recipient: Option<Vec<String>>,
}
