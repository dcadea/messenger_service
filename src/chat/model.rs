use crate::model::Link;
use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::user::model::UserSub;
use crate::util::serialize_object_id;

pub type ChatId = mongodb::bson::oid::ObjectId;

// TODO: revise the necessity of this struct
#[derive(Serialize, Deserialize, Clone)]
pub struct Members {
    pub me: UserSub,
    pub you: UserSub,
}

impl Members {
    pub fn new(me: UserSub, you: UserSub) -> Self {
        Self { me, you }
    }
}

impl Debug for Members {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{} : {}]", self.me, self.you)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<ChatId>,
    pub members: Members,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<String>,
    updated_at: i64,
}

impl Chat {
    pub fn new(members: Members) -> Self {
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
    recipient: UserSub,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_message: Option<String>,
    updated_at: i64,
    links: Vec<Link>,
}

impl ChatDto {
    pub fn from_chat(chat: Chat, recipient: UserSub) -> Self {
        let chat_id = chat.id.clone().expect("No way chat id is missing!?");
        Self {
            id: chat_id,
            recipient: recipient.clone(),
            last_message: chat.last_message.clone(),
            updated_at: chat.updated_at,
            links: vec![
                Link::_self(&format!("/chats/{chat_id}")),
                Link::recipient(&format!("/users?sub={recipient}")),
            ],
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ChatRequest {
    pub recipient: UserSub,
}
