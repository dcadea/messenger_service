use chrono::{DateTime, Utc};
use diesel::prelude::{Associations, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::{
    talk,
    user::{self},
};

use super::Id;

#[derive(Clone, Queryable, Selectable, Identifiable, Associations)]
#[diesel(table_name = crate::schema::messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(user::model::User, foreign_key = owner))]
pub struct Message {
    id: Id,
    talk_id: talk::Id,
    owner: user::Id,
    content: String,
    created_at: DateTime<Utc>,
    seen: bool,
}

impl Message {
    pub const fn new(
        id: Id,
        talk_id: talk::Id,
        owner: user::Id,
        content: String,
        created_at: DateTime<Utc>,
        seen: bool,
    ) -> Self {
        Self {
            id,
            talk_id,
            owner,
            content,
            created_at,
            seen,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::messages)]
pub struct NewMessage<'a> {
    talk_id: &'a talk::Id,
    owner: &'a user::Id,
    content: &'a str,
}

impl<'a> NewMessage<'a> {
    pub const fn new(talk_id: &'a talk::Id, owner: &'a user::Id, content: &'a str) -> Self {
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
    created_at: DateTime<Utc>,
    seen: bool,
}

impl MessageDto {
    pub fn new(talk_id: talk::Id, owner: user::Id, text: impl Into<String>) -> Self {
        Self {
            id: Id::random(),
            talk_id,
            owner,
            content: text.into(),
            created_at: Utc::now().to_utc(),
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

    pub const fn text(&self) -> &str {
        self.content.as_str()
    }

    pub const fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
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
            id: m.id,
            talk_id: m.talk_id,
            owner: m.owner,
            content: m.content,
            created_at: m.created_at,
            seen: m.seen,
        }
    }
}
