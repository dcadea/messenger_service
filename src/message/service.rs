use std::sync::Arc;

use lapin::options::{
    BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, Consumer};
use log::{debug, error};
use serde_json::json;
use tokio::sync::Mutex;

use crate::message::repository::MessageRepository;
use crate::ws::model::Event;

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

    pub async fn publish(&self, body: Event) {
        let message = body.clone().into();

        match self.message_repository.insert(&message).await {
            Ok(_) => {
                debug!("Message saved to database.");

                let queue_name = match self.declare_queue(body.recipient()).await {
                    Ok(name) => name,
                    Err(_) => return,
                };

                let conn = self.rabbitmq_con.lock().await;
                let channel = conn.create_channel().await.unwrap();

                let message_json = json!(message).to_string();

                match channel
                    .basic_publish(
                        "",
                        &queue_name,
                        BasicPublishOptions::default(),
                        message_json.as_bytes(),
                        BasicProperties::default(),
                    )
                    .await
                {
                    Ok(_) => debug!("Message published to queue: {}", queue_name),
                    Err(e) => error!("Failed to publish message: {}", e),
                }
            }
            Err(e) => error!("Failed to save message to database: {}", e),
        }
    }

    pub async fn consume(&self, queue_name: &str) -> Result<(Consumer, Channel), lapin::Error> {
        let queue_name = match self.declare_queue(queue_name).await {
            Ok(name) => name,
            Err(e) => return Err(e),
        };

        let conn = self.rabbitmq_con.lock().await;
        let channel = conn.create_channel().await?;

        let consumer = channel
            .basic_consume(
                &queue_name,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok((consumer, channel))
    }

    pub async fn close_consumer(
        &self,
        consumer_tag: &str,
        channel: Channel,
    ) -> Result<(), lapin::Error> {
        match channel
            .basic_cancel(consumer_tag, BasicCancelOptions::default())
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

    async fn declare_queue(&self, queue_name: &str) -> Result<String, lapin::Error> {
        let conn = self.rabbitmq_con.lock().await;
        let channel = conn.create_channel().await?;

        match channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions {
                    auto_delete: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await
        {
            Ok(queue) => Ok(queue.name().to_string()),
            Err(e) => {
                error!("Failed to declare queue: {}", e);
                Err(e)
            }
        }
    }

    // pub async fn find_by_recipient(
    //     &self,
    //     username: &str,
    // ) -> Result<Vec<Message>, mongodb::error::Error> {
    //     self.message_repository.find_by_recipient(username).await
    // }
}
