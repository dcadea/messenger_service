use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::user;

#[derive(Clone)]
pub struct Ws {
    pub logged_sub: user::Sub,
    channel: Arc<RwLock<lapin::Channel>>,
    pub close: Arc<Notify>,
}

impl Ws {
    pub fn new(logged_sub: user::Sub, channel: lapin::Channel) -> Self {
        Self {
            logged_sub,
            channel: Arc::new(RwLock::new(channel)),
            close: Arc::new(Notify::new()),
        }
    }
}

impl Ws {
    pub async fn get_channel(&self) -> lapin::Channel {
        self.channel.read().await.clone()
    }
}
