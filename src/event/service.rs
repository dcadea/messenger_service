use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt, future::JoinAll};
use log::{debug, error, trace};

use super::{Message, Notification, Subject};

pub type PayloadStream<T> = Pin<Box<dyn Stream<Item = super::Result<T>> + Send>>;

#[async_trait]
pub trait EventService {
    async fn subscribe_event(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Message>>;

    async fn subscribe_noti(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Notification>>;

    async fn publish(&self, s: &Subject<'_>, payload: Bytes);

    async fn publish_many(&self, s: &Subject<'_>, payloads: Vec<Bytes>);
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

#[async_trait]
impl EventService for EventServiceImpl {
    async fn subscribe_event(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Message>> {
        debug!("subscribe <- {s}");
        let subscriber = self.pubsub.subscribe(s).await?;

        let stream = subscriber.then(async |msg| {
            serde_json::from_slice::<Message>(&msg.payload).map_err(super::Error::from)
        });

        Ok(stream.boxed())
    }

    async fn subscribe_noti(&self, s: &Subject<'_>) -> super::Result<PayloadStream<Notification>> {
        debug!("subscribe <- {s}");
        let subscriber = self.pubsub.subscribe(s).await?;

        let stream = subscriber.then(async |msg| {
            serde_json::from_slice::<Notification>(&msg.payload).map_err(super::Error::from)
        });

        Ok(stream.boxed())
    }

    async fn publish(&self, s: &Subject<'_>, payload: Bytes) {
        debug!("publish -> {s}");
        trace!("payload: {payload:#?}");
        if let Err(e) = self.pubsub.publish(s, payload).await {
            error!("failed to publish -> {s}, reason: {e:?}");
        }
    }

    async fn publish_many(&self, s: &Subject<'_>, payloads: Vec<Bytes>) {
        payloads
            .into_iter()
            .map(|p| self.publish(s, p))
            .collect::<JoinAll<_>>()
            .await;
    }
}
