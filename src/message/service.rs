use std::pin::Pin;
use std::str::from_utf8;
use std::sync::Arc;

use futures::StreamExt;
use futures::TryStreamExt;
use lapin::options::{
    BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use log::error;
use serde_json::json;
use tokio::sync::Mutex;
use tokio_stream::Stream;

use crate::message::model::{Message, MessageRequest, MessageResponse};
use crate::state::AppState;

type MessageStream = Pin<Box<dyn Stream<Item = Result<String, lapin::Error>> + Send>>;

const DB_MESSAGES_QUEUE: &str = "db.messages";

pub struct MessageService {
    rabbitmq_con: Arc<Mutex<Connection>>,
}

impl MessageService {
    pub fn new(rabbitmq_con: Arc<Mutex<Connection>>) -> Arc<Self> {
        Self { rabbitmq_con }.into()
    }

    /**
     * Publishes a message to a recipient's dedicated queue.
     */
    pub async fn publish_for_recipient(
        &self,
        request: MessageRequest,
    ) -> Result<MessageResponse, lapin::Error> {
        let message: Message = request.clone().into();
        let response: MessageResponse = message.clone().into();
        self.publish(&request.recipient(), message)
            .await
            .map(|_| response)
    }

    /**
     * Publishes a message to a storage queue.
     */
    pub async fn publish_for_storage(&self, data: String) -> Result<(), lapin::Error> {
        let message = serde_json::from_str::<Message>(&data).unwrap();

        self.publish(DB_MESSAGES_QUEUE, message).await
    }

    async fn publish(&self, queue_name: &str, payload: Message) -> Result<(), lapin::Error> {
        let (queue_name, channel) = self.split_queue(queue_name).await?;
        let message_json = json!(payload).to_string();

        channel
            .basic_publish(
                "",
                &queue_name,
                BasicPublishOptions::default(),
                message_json.as_bytes(),
                BasicProperties::default(),
            )
            .await
            .map(|_| ())
    }

    pub async fn read(
        &self,
        recipient: &str,
    ) -> Result<(String, Channel, MessageStream), lapin::Error> {
        let (queue_name, channel) = self.split_queue(recipient).await?;

        let consumer = channel
            .basic_consume(
                &queue_name,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let consumer_tag = consumer.tag().clone();

        let stream = consumer.and_then(|delivery| {
            let data = from_utf8(&delivery.data)
                .expect("not a utf8 string")
                .to_string();
            async move {
                delivery.ack(BasicAckOptions::default()).await?;
                Ok(data)
            }
        });

        Ok((consumer_tag.to_string(), channel, Box::pin(stream)))
    }

    pub async fn close_consumer(
        &self,
        consumer_tag: String,
        channel: Channel,
    ) -> Result<(), lapin::Error> {
        channel
            .basic_cancel(&consumer_tag, BasicCancelOptions::default())
            .await
            .map(|_| ())
    }

    async fn split_queue(&self, queue_name: &str) -> Result<(String, Channel), lapin::Error> {
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

pub fn start_purging(state: AppState) {
    let message_service_clone = state.message_service.clone();
    tokio::spawn(async move {
        let message_service = message_service_clone.clone();
        let (consumer_tag, channel, messages_stream) =
            match message_service.read(DB_MESSAGES_QUEUE).await {
                Ok(binding) => binding,
                Err(e) => {
                    error!("Failed to read messages: {}", e);
                    return;
                }
            };

        messages_stream
            .for_each(move |data| {
                let message_repository = state.message_repository.clone();
                async move {
                    match data {
                        Ok(data) => {
                            let message: Message =
                                serde_json::from_str(&data).expect("Failed to deserialize message");
                            if let Err(e) = message_repository.insert(&message).await {
                                error!("Failed to store message: {}", e);
                            }
                        }
                        Err(e) => error!("Failed to read message: {}", e),
                    }
                }
            })
            .await;

        if let Err(e) = message_service.close_consumer(consumer_tag, channel).await {
            error!("Failed to close consumer: {}", e);
        };
    });
}
