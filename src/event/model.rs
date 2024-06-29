use std::collections::HashSet;
use std::pin::Pin;

use futures::Stream;
use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use serde::{Deserialize, Serialize};

use crate::chat::model::ChatId;
use crate::message::model::{MessageDto, MessageId};
use crate::user::model::UserSub;

pub(crate) type NotificationStream =
    Pin<Box<dyn Stream<Item = crate::event::Result<Notification>> + Send>>;

pub trait QueueName {
    fn to_string(&self) -> String;
}

pub struct MessagesQueue {
    name: String,
}

impl From<UserSub> for MessagesQueue {
    fn from(user: UserSub) -> Self {
        Self {
            name: user.to_string(),
        }
    }
}

impl QueueName for MessagesQueue {
    fn to_string(&self) -> String {
        format!("messages:{}", self.name)
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
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
    SeenMessage {
        id: MessageId,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    MessageCreated {
        message: MessageDto,
    },
    MessageUpdated {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId,
        text: String,
    },
    MessageDeleted {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId,
    },
    MessageSeen {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId,
    },
    UsersOnline {
        users: HashSet<UserSub>,
    },
}
