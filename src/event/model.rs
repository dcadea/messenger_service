use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::{Notify, RwLock};

use crate::auth::model::UserInfo;
use crate::message::model::MessageId;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Auth { token: String },
    CreateMessage { recipient: String, text: String },
    UpdateMessage { id: MessageId, text: String },
    DeleteMessage { id: MessageId },
}

#[derive(Clone)]
pub struct WsContext {
    user_info: Arc<RwLock<Option<UserInfo>>>,
    pub login: Arc<Notify>,
    pub close: Arc<Notify>,
}

impl WsContext {
    pub fn new() -> Self {
        Self {
            user_info: Arc::new(RwLock::new(None)),
            login: Arc::new(Notify::new()),
            close: Arc::new(Notify::new()),
        }
    }
}

impl WsContext {
    pub async fn set_user_info(&self, user_info: UserInfo) {
        *self.user_info.write().await = Some(user_info);
    }

    pub async fn get_user_info(&self) -> Option<UserInfo> {
        self.user_info.read().await.clone()
    }
}
