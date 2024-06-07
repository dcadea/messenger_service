use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<ChatId>,
    pub members: Members,
    pub last_message: String,
}

impl Chat {
    pub fn new(members: Members, last_message: &str) -> Self {
        Self {
            id: None,
            members,
            last_message: last_message.to_string(),
        }
    }
}

impl From<&ChatRequest> for Chat {
    fn from(chat_request: &ChatRequest) -> Self {
        Self::new(chat_request.clone().members, &chat_request.last_message)
    }
}

#[derive(Serialize)]
pub struct ChatDto {
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    id: ChatId,
    recipient: UserSub,
    last_message: String,
    links: Vec<Link>,
}

impl ChatDto {
    pub fn new(id: ChatId, recipient: UserSub, last_message: &str) -> Self {
        Self {
            id,
            recipient: recipient.clone(),
            last_message: last_message.to_string(),
            links: vec![
                Link::new("self", &format!("/chats/{}", id)),
                Link::new("recipient", &format!("/users?sub={}", recipient)),
            ],
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ChatRequest {
    members: Members,
    last_message: String,
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
