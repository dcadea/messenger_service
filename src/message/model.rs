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
    fn new(sender: &str, recipient: &str, text: &str, timestamp: i64) -> Self {
        Self {
            _id: None,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            text: text.to_string(),
            timestamp,
            seen: false,
        }
    }
}

impl From<MessageRequest> for Message {
    fn from(request: MessageRequest) -> Self {
        Self::new(
            &request.sender,
            &request.recipient,
            &request.text,
            Utc::now().timestamp(),
        )
    }
}

#[derive(Deserialize, Clone)]
pub(super) struct MessageRequest {
    sender: String,
    pub recipient: String,
    text: String,
}

#[derive(Deserialize)]
pub(super) struct MessageParams {
    pub recipient: Option<String>,
}
