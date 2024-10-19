use std::collections::HashSet;
use std::fmt::Display;
use std::pin::Pin;

use futures::Stream;
use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde::{Deserialize, Serialize};

use crate::message::model::MessageDto;
use crate::{chat, message, user};

pub type NotificationStream = Pin<Box<dyn Stream<Item = super::Result<Notification>> + Send>>;

#[derive(Clone)]
pub enum Queue {
    Messages(user::Sub),
}

impl Display for Queue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Queue::Messages(sub) => write!(f, "messages:{sub}"),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    CreateMessage {
        chat_id: chat::Id,
        recipient: user::Sub,
        text: String,
    },
    UpdateMessage {
        id: message::Id,
        text: String,
    },
    DeleteMessage(message::Id),
    MarkAsSeen(message::Id),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    NewMessage {
        message: MessageDto,
    },
    UpdatedMessage {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: message::Id,
        text: String,
    },
    DeletedMessage {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: message::Id,
    },
    SeenMessage {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: message::Id,
    },
    OnlineUsers {
        users: HashSet<user::Sub>,
    },
}
