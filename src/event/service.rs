use std::pin::Pin;
use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use lapin::options::{
    BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use log::error;
use tokio::sync::Mutex;
use tokio_stream::Stream;

use crate::error::ApiError;
use crate::message::model::{Message, MessageRequest};
use crate::message::service::MessageService;
use crate::result::Result;

type MessageStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>>> + Send>>;

const DB_MESSAGES_QUEUE: &str = "db.messages";

#[derive(Clone)]
pub struct EventService {
    rabbitmq_con: Arc<Mutex<Connection>>,
    message_service: Arc<MessageService>,
}

impl EventService {
    pub fn new(rabbitmq_con: Mutex<Connection>, message_service: MessageService) -> Self {
        Self {
            rabbitmq_con: Arc::new(rabbitmq_con),
            message_service: Arc::new(message_service),
        }
    }
}

impl EventService {
    /**
     * Publishes a message to a recipient's dedicated queue.
     */
    pub async fn publish_for_recipient(
        &self,
        sender: &str,
        request: &MessageRequest,
    ) -> Result<()> {
        let message = Message::from_request(sender, request);
        self.publish(&request.recipient, serde_json::to_vec(&message)?.as_slice())
            .await?;
        Ok(())
    }

    /**
     * Publishes a message to a storage queue.
     */
    pub async fn publish_for_storage(&self, data: &[u8]) -> Result<()> {
        self.publish(DB_MESSAGES_QUEUE, data).await?;
        Ok(())
    }

    /**
     * Reads messages from a queue where queue_name is the user's nickname.
     */
    pub async fn read(&self, queue_name: &str) -> Result<(String, Channel, MessageStream)> {
        let (queue_name, channel) = self.split_queue(queue_name).await?;

        let consumer = channel
            .basic_consume(
                &queue_name,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let consumer_tag = consumer.tag().clone();

        let stream = consumer
            .and_then(|delivery| {
                let data = delivery.data.clone();
                async move {
                    delivery.ack(BasicAckOptions::default()).await?;
                    Ok(data)
                }
            })
            .map_err(ApiError::from);

        Ok((consumer_tag.to_string(), channel, Box::pin(stream)))
    }

    /**
     * Closes a consumer by its tag.
     */
    pub async fn close_consumer(&self, consumer_tag: &str, channel: &Channel) -> Result<()> {
        channel
            .basic_cancel(consumer_tag, BasicCancelOptions::default())
            .await?;

        Ok(())
    }

    /**
     * Starts a purging process for the storage queue.
     */
    pub fn start_purging(self) {
        let self_clone = Arc::new(self);
        tokio::spawn(async move {
            let event_service = self_clone.clone();
            let (consumer_tag, channel, messages_stream) =
                match event_service.read(DB_MESSAGES_QUEUE).await {
                    Ok(binding) => binding,
                    Err(e) => {
                        error!("Failed to read messages: {:?}", e);
                        return;
                    }
                };

            messages_stream
                .for_each(move |data| {
                    let message_service = self_clone.message_service.clone();
                    async move {
                        match data {
                            Ok(data) => {
                                let message = serde_json::from_slice::<Message>(&*data)
                                    .expect("Failed to deserialize message");
                                if let Err(e) = message_service.create(&message).await {
                                    error!("Failed to store message: {:?}", e);
                                }
                            }
                            Err(e) => error!("Failed to read message: {:?}", e),
                        }
                    }
                })
                .await;

            if let Err(e) = event_service.close_consumer(&consumer_tag, &channel).await {
                error!("Failed to close consumer: {:?}", e);
            };
        });
    }
}

impl EventService {
    async fn publish(&self, queue_name: &str, payload: &[u8]) -> Result<()> {
        let (queue_name, channel) = self.split_queue(queue_name).await?;

        channel
            .basic_publish(
                "",
                &queue_name,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    async fn split_queue(&self, queue_name: &str) -> Result<(String, Channel)> {
        let conn = self.rabbitmq_con.lock().await;
        let channel = conn.create_channel().await?;

        let queue_name = channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions {
                    auto_delete: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await
            .map(|queue| queue.name().to_string())?;

        Ok((queue_name, channel))
    }
}