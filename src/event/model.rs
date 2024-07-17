use std::collections::HashSet;
use std::fmt::Display;
use std::pin::Pin;

use futures::Stream;
use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde::{Deserialize, Serialize};

use crate::chat::model::ChatId;
use crate::message::model::{MessageDto, MessageId};
use crate::user::model::UserSub;

pub(crate) type EventStream = Pin<Box<dyn Stream<Item = crate::event::Result<Event>> + Send>>;

#[derive(Clone)]
pub enum Queue {
    Messages(UserSub),
}

impl Display for Queue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Queue::Messages(name) => write!(f, "messages:{}", name),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Auth {
        token: String,
    },
    CreateMessage {
        chat_id: ChatId,
        recipient: UserSub,
        text: String,
    },
    UpdateMessage {
        id: MessageId,
        text: String,
    },
    DeleteMessage {
        id: MessageId,
    },
    MarkAsSeenMessage {
        id: MessageId,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    NewMessage {
        message: MessageDto,
    },
    UpdatedMessage {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId,
        text: String,
    },
    DeletedMessage {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId,
    },
    SeenMessage {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId,
    },
    OnlineUsers {
        users: HashSet<UserSub>,
    },
}
