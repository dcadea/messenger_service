use diesel::prelude::{Associations, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    message::{self, model::LastMessage},
    user,
};

use super::{Id, Kind, Picture};

#[derive(Queryable, Selectable, Identifiable, Associations)]
#[diesel(table_name = crate::schema::talks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(message::model::Message, foreign_key = last_message_id))]
pub struct Talk {
    id: Uuid,
    kind: Kind,
    last_message_id: Option<Uuid>,
}

impl Talk {
    pub const fn id(&self) -> &Uuid {
        &self.id
    }

    pub const fn kind(&self) -> &Kind {
        &self.kind
    }

    pub const fn last_message_id(&self) -> Option<&Uuid> {
        self.last_message_id.as_ref()
    }
}

// #[derive(Queryable, Identifiable, Associations)]
// #[diesel(table_name = crate::schema::chats)]
// #[diesel(check_for_backend(diesel::pg::Pg))]
// #[diesel(belongs_to(Talk, foreign_key = id))]
// pub struct Chat {
//     id: Uuid,
// }

// #[derive(Queryable, Identifiable, Associations)]
// #[diesel(table_name = crate::schema::groups)]
// #[diesel(check_for_backend(diesel::pg::Pg))]
// #[diesel(belongs_to(Talk, foreign_key = id))]
// pub struct Group {
//     id: Uuid,
//     owner: Uuid,
//     name: String,
// }

pub struct NewTalk<'a> {
    details: &'a Details,
}

impl<'a> NewTalk<'a> {
    pub fn new(details: &'a Details) -> Self {
        Self { details }
    }

    pub const fn details(&self) -> &Details {
        &self.details
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::chats)]
pub struct NewChat<'a> {
    id: &'a Uuid,
}

impl<'a> NewChat<'a> {
    pub fn new(id: &'a Uuid) -> Self {
        Self { id }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::chats_users)]
pub struct NewChatUser<'a> {
    chat_id: &'a Uuid,
    user_id: &'a Uuid,
}

impl<'a> NewChatUser<'a> {
    pub fn new(chat_id: &'a Uuid, user_id: &'a Uuid) -> Self {
        Self { chat_id, user_id }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::groups)]
pub struct NewGroup<'a> {
    id: &'a Uuid,
    owner: &'a Uuid,
    name: &'a str,
}

impl<'a> NewGroup<'a> {
    pub fn new(id: &'a Uuid, owner: &'a Uuid, name: &'a str) -> Self {
        Self { id, owner, name }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::groups_users)]
pub struct NewGroupUser<'a> {
    group_id: &'a Uuid,
    user_id: &'a Uuid,
}

impl<'a> NewGroupUser<'a> {
    pub fn new(group_id: &'a Uuid, user_id: &'a Uuid) -> Self {
        Self { group_id, user_id }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Details {
    Chat {
        members: [user::Id; 2],
    },
    Group {
        name: String,
        owner: user::Id,
        members: Vec<user::Id>,
    },
}

pub struct TalkWithDetails {}

// impl From<Details> for TalkWithDetails {
//     fn from(details: Details) -> Self {
//         Self {
//             id: Id::random(),
//             details,
//             last_message: None,
//         }
//     }
// }

#[derive(Clone, Serialize, Deserialize)]
pub struct TalkDto {
    id: Id,
    picture: Picture,
    name: String,
    details: DetailsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_message: Option<LastMessage>,
}

impl TalkDto {
    pub fn new(
        id: Id,
        picture: Picture,
        name: impl Into<String>,
        details: DetailsDto,
        last_message: Option<LastMessage>,
    ) -> Self {
        Self {
            id,
            picture,
            name: name.into(),
            details,
            last_message,
        }
    }

    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub fn picture(&self) -> &Picture {
        &self.picture
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn details(&self) -> &DetailsDto {
        &self.details
    }

    pub const fn last_message(&self) -> Option<&LastMessage> {
        self.last_message.as_ref()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DetailsDto {
    Chat {
        sender: user::Id,
        recipient: user::Id,
    },
    Group {
        owner: user::Id,
        sender: user::Id,
    },
}

pub mod pg {
    use diesel::backend::Backend;
    use diesel::deserialize::FromSql;
    use diesel::serialize::{IsNull, Output, ToSql};
    use diesel::sql_types::Text;
    use diesel::{deserialize, serialize};
    use std::io::Write;

    use crate::schema::sql_types::TalkKind;
    use crate::talk::{self, Kind};

    impl<DB> FromSql<TalkKind, DB> for Kind
    where
        DB: Backend,
        String: FromSql<Text, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
            let s = String::from_sql(bytes)?;
            match s.as_str() {
                "chat" => Ok(Kind::Chat),
                "group" => Ok(Kind::Group),
                other => Err(Box::new(talk::Error::UnsupportedKind(other.to_string()))),
            }
        }
    }

    impl ToSql<TalkKind, diesel::pg::Pg> for Kind {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
            match *self {
                Kind::Chat => out.write_all(b"chat")?,
                Kind::Group => out.write_all(b"group")?,
            }
            Ok(IsNull::No)
        }
    }
}
