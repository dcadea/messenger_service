use serde::{Deserialize, Serialize};

use crate::{talk, user};

use super::Id;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Message {
    pub _id: Id,
    pub talk_id: talk::Id,
    pub owner: user::Sub,
    pub text: String,
    pub timestamp: i64,
    pub seen: bool,
}

impl Message {
    pub fn new(talk_id: talk::Id, owner: user::Sub, text: impl Into<String>) -> Self {
        Self {
            _id: Id::random(),
            talk_id,
            owner,
            text: text.into(),
            timestamp: chrono::Utc::now().timestamp(),
            seen: false,
        }
    }

    pub fn with_random_id(&self) -> Self {
        Self {
            _id: Id::random(),
            ..self.clone()
        }
    }

    pub fn with_text(&self, text: &str) -> Self {
        Self {
            text: text.to_string(),
            ..self.clone()
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LastMessage {
    pub id: Id,
    pub text: String,
    pub owner: user::Sub,
    pub timestamp: i64,
    pub seen: bool,
}

impl From<&Message> for LastMessage {
    fn from(msg: &Message) -> Self {
        Self {
            id: msg._id.clone(),
            text: msg.text.clone(),
            owner: msg.owner.clone(),
            timestamp: msg.timestamp,
            seen: msg.seen,
        }
    }
}
