use crate::event::model::Event;
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
    pub fn from_event(sender: &str, event: &Event) -> Option<Self> {
        if let Event::CreateMessage { recipient, text } = event {
            return Some(Self {
                _id: None,
                sender: sender.to_string(),
                recipient: recipient.to_string(),
                text: text.to_string(),
                timestamp: Utc::now().timestamp(),
            });
        }

        None
    }
}

#[derive(Deserialize)]
pub struct MessageParams {
    pub recipient: Option<Vec<String>>,
}
