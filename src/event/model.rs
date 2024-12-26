use std::collections::HashSet;
use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::message::model::Message;
use crate::{message, user};

pub type NotificationStream = Pin<Box<dyn Stream<Item = Option<Notification>> + Send>>;

#[derive(Clone)]
pub enum Queue {
    Messages(user::Sub),
    Notifications(user::Sub),
}

impl async_nats::subject::ToSubject for &Queue {
    fn to_subject(&self) -> async_nats::Subject {
        match self {
            Queue::Messages(sub) => async_nats::Subject::from(format!("messages:{sub}")),
            Queue::Notifications(sub) => async_nats::Subject::from(format!("notifications:{sub}")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    NewMessage { msg: Message },
    UpdatedMessage { id: message::Id, text: String },
    DeletedMessage { id: message::Id },
    SeenMessage { id: message::Id },
    OnlineFriends { friends: HashSet<user::Sub> },
}
