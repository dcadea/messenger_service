use serde::{de::DeserializeOwned, Serialize};
use tokio_stream::StreamExt;

use super::{PayloadStream, Subject};

#[derive(Clone)]
pub struct EventService {
    pubsub: async_nats::Client,
}

impl EventService {
    pub fn new(pubsub: async_nats::Client) -> Self {
        Self { pubsub }
    }
}

impl EventService {
    pub async fn subscribe<T: DeserializeOwned>(
        &self,
        q: &Subject,
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

    pub async fn publish<T: Serialize>(&self, s: &Subject, payload: T) -> super::Result<()> {
        let payload = serde_json::to_vec(&payload)?;
        self.pubsub.publish(s, payload.into()).await?;
        Ok(())
    }
}

// TODO: online users feature
// impl EventService {
//     pub async fn listen_online_status_change(&self) -> super::Result<cache::UpdateStream> {
//         let stream = self
//             .redis
//             .subscribe(&cache::Keyspace::new(cache::Key::UsersOnline))
//             .await?;

//         Ok(stream)
//     }
// }
