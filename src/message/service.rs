use std::sync::Arc;

use lapin::options::{
    BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, Consumer};
use log::{debug, error};
use tokio::sync::Mutex;

use crate::ws::model::Event;

pub struct MessageService {
    rabbitmq_con: Arc<Mutex<Connection>>,
}

impl MessageService {
    pub fn new(rabbitmq_con: Arc<Mutex<Connection>>) -> Self {
        MessageService { rabbitmq_con }
    }

    pub async fn publish(&self, body: Event) {
        let queue_name = match self.declare_queue(body.topic()).await {
            Ok(name) => name,
            Err(_) => return,
        };

        let conn = self.rabbitmq_con.lock().await;
        let channel = conn.create_channel().await.unwrap();

        if let Err(e) = channel
            .basic_publish(
                "",
                &queue_name,
                BasicPublishOptions::default(),
                body.message().as_bytes(),
                BasicProperties::default(),
            )
            .await
        {
            error!("Failed to publish message: {}", e);
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
}
