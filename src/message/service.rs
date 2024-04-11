use std::pin::Pin;
use std::sync::Arc;

use lapin::options::{
    BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use log::{debug, error};
use serde_json::json;
use tokio::sync::Mutex;
use tokio_stream::{Stream, StreamExt};

use crate::message::model::{Message, MessageRequest, MessageResponse};
use crate::message::repository::MessageRepository;

pub struct MessageService {
    rabbitmq_con: Arc<Mutex<Connection>>,
    message_repository: Arc<MessageRepository>,
}

impl MessageService {
    pub fn new(
        rabbitmq_con: Arc<Mutex<Connection>>,
        message_repository: Arc<MessageRepository>,
    ) -> Self {
        MessageService {
            rabbitmq_con,
            message_repository,
        }
    }

    /**
     * Send a message to a recipient.
     */
    pub async fn send(&self, request: MessageRequest) -> Result<MessageResponse, lapin::Error> {
        let message: Message = request.clone().into();
        let (queue_name, channel) = self.split_queue(request.recipient()).await?;
        let message_json = json!(message).to_string();

        channel
            .basic_publish(
                "",
                &queue_name,
                BasicPublishOptions::default(),
                message_json.as_bytes(),
                BasicProperties::default(),
            )
            .await
            .map(|_| message.into())
    }

    pub async fn read(
        &self,
        recipient: &str,
    ) -> Result<
        (
            String, // consumer_tag
            Channel,
            Pin<Box<dyn Stream<Item = Result<String, lapin::Error>> + Send>>,
        ),
        lapin::Error,
    > {
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

        let stream = consumer.then(|delivery| {
            let delivery = delivery.unwrap();
            let data = std::str::from_utf8(&delivery.data).unwrap().to_string();
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
        match channel
            .basic_cancel(&consumer_tag, BasicCancelOptions::default())
            .await
        {
            Ok(_) => {
                debug!("Consumer {} closed", consumer_tag);
                Ok(())
            }
            Err(e) => {
                error!("Failed to close consumer: {}", e);
                Err(e)
            }
        }
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
