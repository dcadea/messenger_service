use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde::{Deserialize, Serialize};

use crate::{chat, user};
use messenger_service::serde::serialize_object_id;

use super::Id;

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    id: Option<Id>,
    chat_id: chat::Id,
    pub owner: user::Sub,
    pub recipient: user::Sub,
    pub text: String,
    timestamp: i64,
    seen: bool,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageDto {
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    pub id: Id,
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    chat_id: chat::Id,
    pub owner: user::Sub,
    pub recipient: user::Sub,
    pub text: String,
    pub timestamp: i64,
    pub seen: bool,
}

impl From<Message> for MessageDto {
    fn from(message: Message) -> Self {
        Self {
            id: message.id.expect("where is message id!?"),
            chat_id: message.chat_id,
            owner: message.owner.clone(),
            recipient: message.recipient.clone(),
            text: message.clone().text,
            timestamp: message.timestamp,
            seen: message.seen,
        }
    }
}
