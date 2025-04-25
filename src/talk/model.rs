use serde::{Deserialize, Serialize};

use crate::{message::model::LastMessage, user};

use super::Id;

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Talk {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    details: Details,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_message: Option<LastMessage>,
}

impl Talk {
    pub fn new(details: Details) -> Self {
        Self {
            id: Id::random(),
            details,
            last_message: None,
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn details(&self) -> &Details {
        &self.details
    }

    pub fn last_message(&self) -> Option<&LastMessage> {
        self.last_message.as_ref()
    }
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
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
