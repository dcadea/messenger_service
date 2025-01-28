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
        s: &Subject,
    ) -> super::Result<PayloadStream<T>> {
        let subscriber = self.pubsub.subscribe(s).await?;

        let stream = subscriber.then(|msg| async move {
            serde_json::from_slice::<T>(&msg.payload).expect("failed payload deserialization")
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
