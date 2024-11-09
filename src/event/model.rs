use std::collections::HashSet;
use std::fmt::Display;
use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::message::model::MessageDto;
use crate::{message, user};

pub type NotificationStream = Pin<Box<dyn Stream<Item = Option<Notification>> + Send>>;

#[derive(Clone)]
pub enum Queue {
    Messages(user::Sub),
}

impl From<user::Sub> for Queue {
    fn from(sub: user::Sub) -> Self {
        Queue::Messages(sub)
    }
}

impl Display for Queue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Queue::Messages(sub) => write!(f, "messages:{sub}"),
        }
    }
}

impl async_nats::subject::ToSubject for &Queue {
    fn to_subject(&self) -> async_nats::Subject {
        match self {
            Queue::Messages(sub) => async_nats::Subject::from(format!("messages:{sub}")),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    MarkAsSeen(message::Id),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    NewMessage { dto: MessageDto },
    UpdatedMessage { id: message::Id, text: String },
    DeletedMessage { id: message::Id },
    SeenMessage { id: message::Id },
    OnlineFriends { friends: HashSet<user::Sub> },
}
