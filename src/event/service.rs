use serde::{de::DeserializeOwned, Serialize};
use tokio_stream::StreamExt;

use crate::integration;
use crate::integration::cache;

use super::{PayloadStream, Queue};

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
    pub async fn subscribe<T: DeserializeOwned>(
        &self,
        q: Queue,
    ) -> super::Result<PayloadStream<T>> {
        let subscriber = self.pubsub.subscribe(q).await?;

        let stream = subscriber.then(|msg| async move {
            match serde_json::from_slice::<T>(&msg.payload) {
                Ok(noti) => Some(noti),
                Err(e) => {
                    log::error!("failed to deserialize notification: {:?}", e);
                    None
                }
            }
        });

        Ok(Box::pin(stream))
    }

    pub async fn publish<T: Serialize>(&self, q: Queue, payload: T) -> super::Result<()> {
        let payload = serde_json::to_vec(&payload)?;
        self.pubsub.publish(q, payload.into()).await?;
        Ok(())
    }
}

impl EventService {
    pub async fn listen_online_status_change(&self) -> super::Result<cache::UpdateStream> {
        let stream = self
            .redis
            .subscribe(&cache::Keyspace::new(cache::Key::UsersOnline))
            .await?;

        Ok(stream)
    }
}
