use serde::{Deserialize, Serialize};

use crate::{chat, user};

use super::Id;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Message {
    #[serde(alias = "_id")]
    pub id: Id,
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
            id: Id::random(),
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
            id: Id::random(),
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
