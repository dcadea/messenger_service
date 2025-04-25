use serde::{Deserialize, Serialize};

use crate::{talk, user};

use super::Id;

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq, Debug)]
pub struct Message {
    #[serde(rename = "_id")]
    id: Id,
    talk_id: talk::Id,
    owner: user::Sub,
    text: String,
    timestamp: i64,
    seen: bool,
}

impl Message {
    pub fn new(talk_id: talk::Id, owner: user::Sub, text: impl Into<String>) -> Self {
        Self {
            id: Id::random(),
            talk_id,
            owner,
            text: text.into(),
            timestamp: chrono::Utc::now().timestamp(),
            seen: false,
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn talk_id(&self) -> &talk::Id {
        &self.talk_id
    }

    pub fn owner(&self) -> &user::Sub {
        &self.owner
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn seen(&self) -> bool {
        self.seen
    }

    pub fn with_random_id(&self) -> Self {
        Self {
            id: Id::random(),
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

#[cfg(test)]
impl Message {
    pub fn set_timestamp(&mut self, timestamp: i64) {
        self.timestamp = timestamp;
    }

    pub fn set_seen(&mut self, seen: bool) {
        self.seen = seen;
    }
}

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq, Debug)]
pub struct LastMessage {
    id: Id,
    text: String,
    owner: user::Sub,
    timestamp: i64,
    seen: bool,
}

impl LastMessage {
    pub fn new(
        id: Id,
        text: impl Into<String>,
        owner: user::Sub,
        timestamp: i64,
        seen: bool,
    ) -> Self {
        Self {
            id,
            text: text.into(),
            owner,
            timestamp,
            seen,
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn owner(&self) -> &user::Sub {
        &self.owner
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn seen(&self) -> bool {
        self.seen
    }
}

impl From<&Message> for LastMessage {
    fn from(msg: &Message) -> Self {
        Self {
            id: msg.id.clone(),
            text: msg.text.clone(),
            owner: msg.owner.clone(),
            timestamp: msg.timestamp,
            seen: msg.seen,
        }
    }
}
