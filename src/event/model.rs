use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};

use crate::chat::model::ChatId;
use crate::message::model::{MessageDto, MessageId};
use crate::user::model::{UserInfo, UserSub};

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
    Auth { token: String },
    CreateMessage { chat_id: ChatId, recipient: UserSub, text: String },
    UpdateMessage { id: MessageId, text: String },
    DeleteMessage { id: MessageId },
    SeenMessage { id: MessageId },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Notification {
    MessageCreated { message: MessageDto },
    MessageUpdated {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId,
        text: String,
    },
    MessageDeleted {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId
    },
    MessageSeen {
        #[serde(serialize_with = "serialize_object_id_as_hex_string")]
        id: MessageId
    },
}

#[derive(Clone)]
pub struct WsCtx {
    user_info: Arc<RwLock<Option<UserInfo>>>,
    pub login: Arc<Notify>,
    pub close: Arc<Notify>,
}

impl WsCtx {
    pub fn new() -> Self {
        Self {
            user_info: Arc::new(RwLock::new(None)),
            login: Arc::new(Notify::new()),
            close: Arc::new(Notify::new()),
        }
    }
}

impl WsCtx {
    pub async fn set_user_info(&self, user_info: UserInfo) {
        *self.user_info.write().await = Some(user_info);
    }

    pub async fn get_user_info(&self) -> Option<UserInfo> {
        self.user_info.read().await.clone()
    }
}
