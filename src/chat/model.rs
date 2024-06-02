use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde::{Deserialize, Serialize};

use crate::user::model::UserSub;
use crate::util::serialize_object_id;

pub type ChatId = mongodb::bson::oid::ObjectId;

// TODO: revise the necessity of this struct
#[derive(Serialize, Deserialize, Clone)]
pub struct Members {
    pub me: UserSub,
    pub you: UserSub,
}

impl Members {
    pub fn new(me: UserSub, you: UserSub) -> Self {
        Self { me, you }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_object_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<ChatId>,
    pub members: Members,
    pub last_message: String,
}

impl Chat {
    pub fn new(members: Members, last_message: &str) -> Self {
        Self {
            id: None,
            members,
            last_message: last_message.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct ChatTO {
    #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    pub id: ChatId,
    pub recipient: UserSub,
    pub last_message: String,
}
