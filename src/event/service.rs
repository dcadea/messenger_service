use bytes::Bytes;
use futures::StreamExt;
use log::error;

use super::{Message, Notification, PayloadStream, Subject};

#[async_trait::async_trait]
pub trait EventService {
    async fn subscribe_event(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Message>>;

    async fn subscribe_noti(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Notification>>;

    async fn publish(&self, s: &Subject<'_>, payload: Bytes);

    async fn publish_all(&self, s: &Subject<'_>, payloads: Vec<Bytes>);
}

#[derive(Clone)]
pub struct EventServiceImpl {
    pubsub: async_nats::Client,
}

impl EventServiceImpl {
    pub fn new(pubsub: async_nats::Client) -> Self {
        Self { pubsub }
    }
}

#[async_trait::async_trait]
impl EventService for EventServiceImpl {
    async fn subscribe_event(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Message>> {
        let subscriber = self.pubsub.subscribe(s).await?;

        let stream = subscriber.then(async |msg| {
            // FIXME: expect!
            serde_json::from_slice::<Message>(&msg.payload)
                .expect("failed event message deserialization")
        });

        Ok(stream.boxed())
    }

    async fn subscribe_noti(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Notification>> {
        let subscriber = self.pubsub.subscribe(s).await?;

        let stream = subscriber.then(async |msg| {
            // FIXME: expect!
            serde_json::from_slice::<Notification>(&msg.payload)
                .expect("failed notification deserialization")
        });

        Ok(stream.boxed())
    }

    async fn publish(&self, s: &Subject<'_>, payload: Bytes) {
        if let Err(e) = self.pubsub.publish(s, payload).await {
            error!("failed to publish into subject: {s:?}, reason: {e:?}");
        }
    }

    async fn publish_all(&self, s: &Subject<'_>, payloads: Vec<Bytes>) {
        for p in payloads.into_iter() {
            self.publish(s, p).await;
        }
    }
}
