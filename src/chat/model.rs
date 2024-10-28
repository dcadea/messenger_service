use serde::{Deserialize, Serialize};

use crate::user;

use super::Id;

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(alias = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    pub members: [user::Sub; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<String>,
    updated_at: i64,
}

impl Chat {
    pub fn new(members: [user::Sub; 2]) -> Self {
        Self {
            id: None,
            members,
            last_message: None,
            updated_at: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Serialize)]
pub struct ChatDto {
    pub id: Id,
    pub recipient: user::Sub,
    pub recipient_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<String>,
    updated_at: i64,
}

impl ChatDto {
    pub fn new(chat: Chat, recipient: user::Sub, recipient_name: String) -> Self {
        let chat_id = chat.id.expect("No way chat id is missing!?");
        Self {
            id: chat_id,
            recipient,
            recipient_name,
            last_message: chat.last_message,
            updated_at: chat.updated_at,
        }
    }
}
