use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::event::error::EventError;
use crate::event::Result;
use crate::user::model::UserInfo;

#[derive(Clone)]
pub struct Ws {
    user_info: Arc<RwLock<Option<UserInfo>>>,
    channel: Arc<RwLock<Option<lapin::Channel>>>,
    pub login: Arc<Notify>,
    pub close: Arc<Notify>,
}

impl Ws {
    pub fn new() -> Self {
        Self {
            user_info: Arc::new(RwLock::new(None)),
            channel: Arc::new(RwLock::new(None)),
            login: Arc::new(Notify::new()),
            close: Arc::new(Notify::new()),
        }
    }
}

impl Ws {
    pub async fn set_user_info(&self, user_info: UserInfo) {
        *self.user_info.write().await = Some(user_info);
    }

    pub async fn get_user_info(&self) -> Option<UserInfo> {
        self.user_info.read().await.clone()
    }

    pub async fn set_channel(&self, channel: lapin::Channel) {
        *self.channel.write().await = Some(channel);
    }

    pub async fn get_channel(&self) -> Result<lapin::Channel> {
        self.channel
            .read()
            .await
            .clone()
            .ok_or(EventError::MissingAmqpChannel)
    }
}
