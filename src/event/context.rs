use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::user;
use crate::user::model::UserInfo;

#[derive(Clone)]
pub struct Ws {
    pub user_info: Arc<UserInfo>,
    channel: Arc<RwLock<lapin::Channel>>,
    online_friends: Arc<RwLock<HashSet<user::Sub>>>,
    pub close: Arc<Notify>,
}

impl Ws {
    pub fn new(user_info: UserInfo, channel: lapin::Channel) -> Self {
        Self {
            user_info: Arc::new(user_info),
            channel: Arc::new(RwLock::new(channel)),
            online_friends: Arc::new(RwLock::new(HashSet::new())),
            close: Arc::new(Notify::new()),
        }
    }
}

impl Ws {
    pub async fn get_channel(&self) -> lapin::Channel {
        self.channel.read().await.clone()
    }

    pub async fn set_online_friends(&self, friends: HashSet<user::Sub>) {
        *self.online_friends.write().await = friends;
    }

    pub async fn same_online_friends(&self, friends: &HashSet<user::Sub>) -> bool {
        let f = self.online_friends.read().await;
        f.symmetric_difference(friends).count() == 0
    }
}
