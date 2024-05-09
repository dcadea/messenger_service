use bson::oid::ObjectId;
use chrono::Utc;
use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    #[serde(skip)]
    _id: Option<ObjectId>,
    sender: String,
    pub recipient: String,
    text: String,
    timestamp: i64,
}

impl Message {
    pub fn from_request(sender: &str, request: &MessageRequest) -> Self {
        Self {
            _id: None,
            sender: sender.to_string(),
            recipient: request.clone().recipient,
            text: request.clone().text,
            timestamp: Utc::now().timestamp(),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct MessageRequest {
    pub recipient: String,
    text: String,
}

#[derive(Deserialize)]
pub struct MessageParams {
    pub recipient: Option<Vec<String>>,
}
