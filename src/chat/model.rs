use serde::{Deserialize, Serialize};

use crate::message::model::LastMessage;
use crate::user;

use super::Id;
use super::Kind;

#[derive(Clone, Serialize, Deserialize)]
pub struct Chat {
    pub _id: Id,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<user::Sub>,
    pub members: Vec<user::Sub>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<LastMessage>,
}

impl Chat {
    pub fn private(members: [user::Sub; 2]) -> Self {
        Self {
            _id: Id::random(),
            kind: Kind::Private,
            owner: None,
            members: members.to_vec(),
            last_message: None,
        }
    }

    #[allow(dead_code)] // TODO
    pub fn group(owner: user::Sub, members: Vec<user::Sub>) -> Self {
        Self {
            _id: Id::random(),
            kind: Kind::Group,
            owner: Some(owner),
            members,
            last_message: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatDto {
    pub id: Id,
    pub sender: user::Sub,
    pub recipient: user::Sub,
    pub recipient_picture: String,
    pub recipient_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<LastMessage>,
}

impl ChatDto {
    pub fn new(
        chat: Chat,
        sender: user::Sub,
        recipient: user::Sub,
        recipient_picture: impl Into<String>,
        recipient_name: impl Into<String>,
    ) -> Self {
        Self {
            id: chat._id,
            sender,
            recipient,
            recipient_picture: recipient_picture.into(),
            recipient_name: recipient_name.into(),
            last_message: chat.last_message,
        }
    }
}
