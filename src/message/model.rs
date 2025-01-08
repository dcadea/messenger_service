use serde::{Deserialize, Serialize};

use crate::{chat, user};

use super::Id;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Message {
    pub _id: Id,
    pub chat_id: chat::Id,
    pub owner: user::Sub,
    pub recipient: user::Sub,
    pub text: String,
    pub timestamp: i64,
    pub seen: bool,
}

impl Message {
    pub fn new(chat_id: chat::Id, owner: user::Sub, recipient: user::Sub, text: &str) -> Self {
        Self {
            _id: Id::random(),
            chat_id,
            owner,
            recipient,
            text: text.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            seen: false,
        }
    }

    pub fn with_random_id(&self) -> Self {
        Self {
            _id: Id::random(),
            ..self.clone()
        }
    }

    pub fn with_text(&self, text: &str) -> Self {
        Self {
            text: text.to_string(),
            ..self.clone()
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LastMessage {
    pub id: Id,
    pub text: String,
    pub timestamp: i64,
}

impl From<&Message> for LastMessage {
    fn from(msg: &Message) -> Self {
        Self {
            id: msg._id.clone(),
            text: msg.text.clone(),
            timestamp: msg.timestamp,
        }
    }
}
