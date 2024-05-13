use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::{Notify, RwLock};

use crate::auth::model::UserInfo;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventRequest {
    Auth { token: String },
    CreateMessage { recipient: String, text: String },
}

#[derive(Clone)]
pub struct WsRequestContext {
    user_info: Arc<RwLock<Option<UserInfo>>>,
    pub login: Arc<Notify>,
    pub close: Arc<Notify>,
}

impl WsRequestContext {
    pub fn new() -> Self {
        Self {
            user_info: Arc::new(RwLock::new(None)),
            login: Arc::new(Notify::new()),
            close: Arc::new(Notify::new()),
        }
    }
}

impl WsRequestContext {
    pub async fn set_user_info(&self, user_info: UserInfo) {
        *self.user_info.write().await = Some(user_info);
    }

    pub async fn get_user_info(&self) -> Option<UserInfo> {
        self.user_info.read().await.clone()
    }
}
