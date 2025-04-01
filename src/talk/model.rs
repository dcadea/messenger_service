use serde::{Deserialize, Serialize};

use crate::{message::model::LastMessage, user};

use super::{Id, Kind};

#[derive(Clone, Serialize, Deserialize)]
pub struct Talk {
    #[serde(rename = "_id")]
    pub id: Id,
    pub kind: Kind,
    pub details: Details,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<LastMessage>,
}

impl Talk {
    pub fn new(details: Details) -> Self {
        let kind = match details {
            Details::Chat { .. } => Kind::Chat,
            Details::Group { .. } => Kind::Group,
        };

        Self {
            id: Id::random(),
            kind,
            details,
            last_message: None,
        }
    }
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
