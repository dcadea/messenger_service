use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde;
use serde::{Deserialize, Serialize};

use crate::model::Link;
use crate::user::model::Sub;
use crate::util::serialize_object_id;

pub type ChatId = mongodb::bson::oid::ObjectId;

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<ChatId>,
    pub members: [Sub; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<String>,
    updated_at: i64,
}

impl Chat {
    pub fn new(members: [Sub; 2]) -> Self {
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
    pub id: ChatId,
    pub recipient: Sub,
    recipient_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_message: Option<String>,
    updated_at: i64,
    links: Vec<Link>,
}

impl ChatDto {
    pub fn new(chat: Chat, recipient: Sub, recipient_name: String) -> Self {
        let chat_id = chat.id.expect("No way chat id is missing!?");
        Self {
            id: chat_id,
            recipient,
            recipient_name,
            last_message: chat.last_message.clone(),
            updated_at: chat.updated_at,
            links: vec![],
        }
    }

    pub fn with_links(mut self, links: Vec<Link>) -> Self {
        self.links = links;
        self
    }
}

#[derive(Deserialize, Clone)]
pub struct ChatRequest {
    pub recipient: Sub,
}
