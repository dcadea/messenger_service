use chrono::{NaiveDateTime, Utc};
use diesel::prelude::{Associations, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    talk,
    user::{self},
};

use super::Id;

#[derive(Queryable, Selectable, Identifiable, Associations)]
#[diesel(table_name = crate::schema::messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(user::model::User, foreign_key = owner))]
pub struct Message {
    id: Uuid,
    talk_id: Uuid,
    owner: Uuid,
    content: String,
    created_at: NaiveDateTime,
    seen: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::messages)]
pub struct NewMessage<'a> {
    talk_id: &'a Uuid,
    owner: &'a Uuid,
    content: &'a str,
}

impl<'a> NewMessage<'a> {
    pub fn new(talk_id: &'a Uuid, owner: &'a Uuid, content: &'a str) -> Self {
        Self {
            talk_id,
            owner,
            content,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct MessageDto {
    id: Id,
    talk_id: talk::Id,
    owner: user::Id,
    content: String,
    created_at: NaiveDateTime,
    seen: bool,
}

impl MessageDto {
    pub fn new(talk_id: talk::Id, owner: user::Id, text: impl Into<String>) -> Self {
        Self {
            id: Id::random(),
            talk_id,
            owner,
            content: text.into(),
            created_at: Utc::now().naive_utc(),
            seen: false,
        }
    }

    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub const fn talk_id(&self) -> &talk::Id {
        &self.talk_id
    }

    pub const fn owner(&self) -> &user::Id {
        &self.owner
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub const fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }

    pub const fn seen(&self) -> bool {
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
            content: text.to_string(),
            ..self.clone()
        }
    }
}

impl From<Message> for MessageDto {
    fn from(m: Message) -> Self {
        Self {
            id: Id(m.id),
            talk_id: talk::Id(m.talk_id),
            owner: user::Id(m.owner),
            content: m.content,
            created_at: m.created_at,
            seen: m.seen,
        }
    }
}

#[cfg(test)]
impl MessageDto {
    pub const fn set_timestamp(&mut self, timestamp: NaiveDateTime) {
        self.created_at = timestamp;
    }

    pub const fn set_seen(&mut self, seen: bool) {
        self.seen = seen;
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct LastMessage {
    id: Id,
    text: String,
    owner: user::Id,
    timestamp: NaiveDateTime,
    seen: bool,
}

impl LastMessage {
    pub fn new(
        id: Id,
        text: impl Into<String>,
        owner: user::Id,
        timestamp: NaiveDateTime,
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

    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub const fn owner(&self) -> &user::Id {
        &self.owner
    }

    pub const fn timestamp(&self) -> NaiveDateTime {
        self.timestamp
    }

    pub const fn seen(&self) -> bool {
        self.seen
    }
}

impl From<&MessageDto> for LastMessage {
    fn from(msg: &MessageDto) -> Self {
        Self {
            id: msg.id.clone(),
            text: msg.content.clone(),
            owner: msg.owner.clone(),
            timestamp: msg.created_at,
            seen: msg.seen,
        }
    }
}
