use serde::{Deserialize, Serialize};

use crate::{message::model::LastMessage, user};

use super::Id;

#[derive(Clone, Serialize, Deserialize)]
pub struct Talk {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub details: Details,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<LastMessage>,
}

impl Talk {
    pub fn new(details: Details) -> Self {
        Self {
            id: Id::random(),
            details,
            last_message: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "details", rename_all = "snake_case")]
pub enum Details {
    Chat {
        members: [user::Sub; 2],
    },
    Group {
        name: String,
        picture: String,
        owner: user::Sub,
        members: Vec<user::Sub>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TalkDto {
    pub id: Id,
    pub picture: String,
    pub name: String,
    pub details: DetailsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<LastMessage>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DetailsDto {
    Chat {
        sender: user::Sub,
        recipient: user::Sub,
    },
    Group,
}
