use anyhow::Context;
use tokio_stream::StreamExt;

use crate::integration;
use crate::integration::cache;

use super::model::{Notification, NotificationStream, Queue};

#[derive(Clone)]
pub struct EventService {
    pubsub: async_nats::Client,
    redis: integration::cache::Redis,
}

impl EventService {
    pub fn new(pubsub: async_nats::Client, redis: integration::cache::Redis) -> Self {
        Self { pubsub, redis }
    }
}

impl EventService {
    pub async fn read(&self, q: &Queue) -> super::Result<NotificationStream> {
        let subscriber = self.pubsub.subscribe(q).await?;

        let stream = subscriber.then(|msg| async move {
            match serde_json::from_slice::<Notification>(&msg.payload) {
                Ok(noti) => Some(noti),
                Err(e) => {
                    log::error!("failed to deserialize notification: {:?}", e);
                    None
                }
            }
        });

        Ok(Box::pin(stream))
    }

    pub async fn publish_noti(&self, q: &Queue, noti: &Notification) -> super::Result<()> {
        let payload = serde_json::to_vec(noti)?;
        self.pubsub.publish(q, payload.into()).await?;
        Ok(())
    }
}

impl EventService {
    pub async fn listen_online_status_change(&self) -> anyhow::Result<cache::UpdateStream> {
        let stream = self
            .redis
            .subscribe(&cache::Keyspace::new(cache::Key::UsersOnline))
            .await
            .with_context(|| "Failed to subscribe to online status change")?;

        Ok(stream)
    }
}
