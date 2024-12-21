use serde::{Deserialize, Serialize};

use crate::{chat, user};

use super::Id;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Message {
    #[serde(alias = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
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
            id: None,
            chat_id,
            owner,
            recipient,
            text: text.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            seen: false,
        }
    }

    pub fn with_id(&self, id: Id) -> Self {
        Self {
            id: Some(id),
            ..self.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageDto {
    pub id: Id,
    pub chat_id: chat::Id,
    pub owner: user::Sub,
    pub recipient: user::Sub,
    pub text: String,
    pub timestamp: i64,
    pub seen: bool,
}

impl From<&Message> for MessageDto {
    fn from(message: &Message) -> Self {
        Self {
            id: message.id.clone().expect("where is message id!?"),
            chat_id: message.chat_id.clone(),
            owner: message.owner.clone(),
            recipient: message.recipient.clone(),
            text: message.clone().text,
            timestamp: message.timestamp,
            seen: message.seen,
        }
    }
}
