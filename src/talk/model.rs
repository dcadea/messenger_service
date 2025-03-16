use serde::{Deserialize, Serialize};

use crate::{message::model::LastMessage, user};

use super::Id;

#[derive(Clone, Serialize, Deserialize)]
pub struct Talk {
    pub _id: Id,
    pub details: Details,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<LastMessage>,
}

#[derive(Clone, Serialize, Deserialize)]
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
pub enum DetailsDto {
    Chat {
        sender: user::Sub,
        recipient: user::Sub,
    },
    Group,
}
