use messenger_service::serde::serialize_object_id;
use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde;
use serde::{Deserialize, Serialize};

use crate::user;

use super::Id;

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
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
            updated_at: 0,
        }
    }
}

#[derive(Serialize)]
pub struct ChatDto {
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    pub id: Id,
    pub recipient: user::Sub,
    recipient_name: String,
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

#[derive(Deserialize, Clone)]
pub struct ChatRequest {
    pub recipient: user::Sub,
}
