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
    pub last_message: Option<String>,
}

impl Chat {
    pub fn new(members: Members) -> Self {
        Self {
            id: None,
            members,
            last_message: None,
        }
    }
}

#[derive(Serialize)]
pub struct ChatDto {
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    id: ChatId,
    recipient: UserSub,
    last_message: Option<String>,
    links: Vec<Link>,
}

impl ChatDto {
    pub fn new(id: ChatId, recipient: UserSub, last_message: Option<String>) -> Self {
        Self {
            id,
            recipient: recipient.clone(),
            last_message,
            links: vec![
                Link::new("self", &format!("/chats/{}", id)),
                Link::new("recipient", &format!("/users?sub={}", recipient)),
            ],
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ChatRequest {
    pub recipient: UserSub,
}

// TODO: extract into common module
#[derive(Serialize)]
struct Link {
    rel: String, // TODO: create a enum for this field
    href: String,
}

impl Link {
    pub fn new(rel: &str, path: &str) -> Self {
        Self {
            rel: rel.to_string(),
            // TODO: get the base url from configuration
            href: format!("http://127.0.0.1:8000/api/v1{}", path),
        }
    }
}
