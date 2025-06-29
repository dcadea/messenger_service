use chrono::NaiveDateTime;
use diesel::{
    prelude::{Associations, Identifiable, Insertable, Queryable, QueryableByName, Selectable},
    sql_types,
};
use serde::{Deserialize, Serialize};

use crate::{
    message::{
        self,
        model::{Message, MessageDto},
    },
    user,
};

use super::{Id, Kind, Picture};

#[derive(Queryable, Selectable, Identifiable, Associations)]
#[diesel(table_name = crate::schema::talks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(message::model::Message, foreign_key = last_message_id))]
pub struct Talk {
    id: Id,
    kind: Kind,
    last_message_id: Option<message::Id>,
}

impl Talk {
    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub const fn kind(&self) -> &Kind {
        &self.kind
    }

    pub const fn last_message_id(&self) -> Option<&message::Id> {
        self.last_message_id.as_ref()
    }
}

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
    id: &'a Id,
}

impl<'a> NewChat<'a> {
    pub fn new(id: &'a Id) -> Self {
        Self { id }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::chats_users)]
pub struct NewChatUser<'a> {
    chat_id: &'a Id,
    user_id: &'a user::Id,
}

impl<'a> NewChatUser<'a> {
    pub fn new(chat_id: &'a Id, user_id: &'a user::Id) -> Self {
        Self { chat_id, user_id }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::groups)]
pub struct NewGroup<'a> {
    id: &'a Id,
    owner: &'a user::Id,
    name: &'a str,
}

impl<'a> NewGroup<'a> {
    pub fn new(id: &'a Id, owner: &'a user::Id, name: &'a str) -> Self {
        Self { id, owner, name }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::groups_users)]
pub struct NewGroupUser<'a> {
    group_id: &'a Id,
    user_id: &'a user::Id,
}

impl<'a> NewGroupUser<'a> {
    pub fn new(group_id: &'a Id, user_id: &'a user::Id) -> Self {
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

pub struct ChatTalk {
    id: Id,
    last_message: Option<Message>,
    recipient: user::Id,
    name: String, // TODO: implement FromSql and ToSql for nickname and picture
    picture: String,
}

impl ChatTalk {
    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub const fn last_message(&self) -> Option<&Message> {
        self.last_message.as_ref()
    }

    pub const fn recipient(&self) -> &user::Id {
        &self.recipient
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn picture(&self) -> &str {
        &self.picture
    }
}

impl From<ChatWithLastMessage> for ChatTalk {
    fn from(c: ChatWithLastMessage) -> Self {
        let last_message = if c.message_id.is_some() {
            Some(Message::new(
                c.message_id.expect("message_id should be present"),
                c.id.clone(),
                c.owner.expect("owner should be present"),
                c.content.expect("content should be present"),
                c.created_at.expect("created_at should be present"),
                c.seen.expect("seen should be present"),
            ))
        } else {
            None
        };

        Self {
            id: c.id,
            last_message,
            recipient: c.recipient,
            name: c.name,
            picture: c.picture,
        }
    }
}

#[derive(QueryableByName, Debug)]
pub(super) struct ChatWithLastMessage {
    #[diesel(sql_type = sql_types::Uuid)]
    id: Id,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Uuid>)]
    message_id: Option<message::Id>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Uuid>)]
    owner: Option<user::Id>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    content: Option<String>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Bool>)]
    seen: Option<bool>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Timestamp>)]
    created_at: Option<NaiveDateTime>,
    #[diesel(sql_type = sql_types::Uuid)]
    recipient: user::Id,
    #[diesel(sql_type = sql_types::Text)]
    name: String,
    #[diesel(sql_type = sql_types::Text)]
    picture: String,
}

pub struct GroupTalk {
    id: Id,
    last_message: Option<Message>,
    owner: user::Id,
    name: String,
}

impl GroupTalk {
    pub fn new(id: Id, last_message: Option<Message>, owner: user::Id, name: String) -> Self {
        Self {
            id,
            last_message,
            owner,
            name,
        }
    }

    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub const fn last_message(&self) -> Option<&Message> {
        self.last_message.as_ref()
    }

    pub const fn owner(&self) -> &user::Id {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TalkDto {
    id: Id,
    picture: Picture,
    name: String,
    details: DetailsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_message: Option<MessageDto>,
}

impl TalkDto {
    pub fn new(
        id: Id,
        picture: Picture,
        name: impl Into<String>,
        details: DetailsDto,
        last_message: Option<MessageDto>,
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

    pub const fn last_message(&self) -> Option<&MessageDto> {
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
