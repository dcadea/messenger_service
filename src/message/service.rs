use std::sync::Arc;

use lapin::options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Connection, Consumer};
use log::error;
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
        let conn = self.rabbitmq_con.lock().await;
        let channel = conn.create_channel().await.unwrap();

        let queue = match channel
            .queue_declare(
                body.topic(),
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
        {
            Ok(queue) => queue,
            Err(e) => {
                error!("Failed to declare queue: {}", e);
                return;
            }
        };

        if let Err(e) = channel
            .basic_publish(
                "",
                queue.name().as_str(),
                BasicPublishOptions::default(),
                body.message().as_bytes(),
                BasicProperties::default(),
            )
            .await
        {
            error!("Failed to publish message: {}", e);
        }
    }

    pub async fn consume(&self, queue_name: &str) -> Result<Consumer, lapin::Error> {
        let conn = self.rabbitmq_con.lock().await;
        let channel = conn.create_channel().await?;

        let consumer = channel
            .basic_consume(
                queue_name,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(consumer)
    }
}
