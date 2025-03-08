use futures::StreamExt;
use log::error;
use serde::{Serialize, de::DeserializeOwned};

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
        s: &Subject<'_>,
    ) -> super::Result<PayloadStream<T>> {
        let subscriber = self.pubsub.subscribe(s).await?;

        let stream = subscriber.then(async |msg| {
            // FIXME: expect!
            serde_json::from_slice::<T>(&msg.payload).expect("failed payload deserialization")
        });

        Ok(stream.boxed())
    }

    pub async fn publish<T: Serialize>(&self, s: &Subject<'_>, payload: &T) {
        if let Err(e) = serde_json::to_vec(payload)
            .map(async |payload| self.pubsub.publish(s, payload.into()).await)
        {
            error!("failed to publish into subject: {s:?}, reason: {e:?}");
        }
    }

    pub async fn publish_all<T: Serialize>(&self, s: &Subject<'_>, payloads: &[T]) {
        for p in payloads {
            self.publish(s, p).await;
        }
    }
}
