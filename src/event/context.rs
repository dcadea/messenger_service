use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::user::model::UserInfo;
use crate::{event, user};

#[derive(Clone)]
pub struct Ws {
    user_info: Arc<RwLock<Option<UserInfo>>>,
    channel: Arc<RwLock<Option<lapin::Channel>>>,
    online_friends: Arc<RwLock<HashSet<user::Sub>>>,
    pub login: Arc<Notify>,
    pub close: Arc<Notify>,
}

impl Ws {
    pub fn new() -> Self {
        Self {
            user_info: Arc::new(RwLock::new(None)),
            channel: Arc::new(RwLock::new(None)),
            online_friends: Arc::new(RwLock::new(HashSet::new())),
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

    pub async fn get_channel(&self) -> super::Result<lapin::Channel> {
        self.channel
            .read()
            .await
            .clone()
            .ok_or(event::Error::MissingAmqpChannel)
    }

    pub async fn set_online_friends(&self, friends: HashSet<user::Sub>) {
        *self.online_friends.write().await = friends;
    }

    pub async fn same_online_friends(&self, friends: &HashSet<user::Sub>) -> bool {
        let f = self.online_friends.read().await;
        f.symmetric_difference(friends).count() == 0
    }
}
