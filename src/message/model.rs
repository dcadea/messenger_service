use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde::{Deserialize, Serialize};

use crate::chat::model::ChatId;
use crate::user::model::Sub;
use crate::util::serialize_object_id;

pub type MessageId = mongodb::bson::oid::ObjectId;

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    id: Option<MessageId>,
    chat_id: ChatId,
    pub owner: Sub,
    pub recipient: Sub,
    pub text: String,
    timestamp: i64,
    seen: bool,
}

impl Message {
    pub fn new(chat_id: ChatId, owner: Sub, recipient: Sub, text: &str) -> Self {
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

    pub fn with_id(&self, id: MessageId) -> Self {
        Self {
            id: Some(id),
            chat_id: self.chat_id,
            owner: self.owner.clone(),
            recipient: self.recipient.clone(),
            text: self.text.clone(),
            timestamp: self.timestamp,
            seen: self.seen,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageDto {
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    id: MessageId,
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    chat_id: ChatId,
    owner: Sub,
    recipient: Sub,
    text: String,
    timestamp: i64,
    seen: bool,
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

#[derive(Deserialize)]
pub struct MessageParams {
    pub chat_id: Option<ChatId>,
    pub end_time: Option<i64>,
    pub limit: Option<usize>,
}
