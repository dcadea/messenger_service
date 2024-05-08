use std::pin::Pin;
use std::str::from_utf8;
use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
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

use crate::error::ApiError;
use crate::message::model::{Message, MessageRequest};
use crate::message::repository::MessageRepository;
use crate::result::Result;

type MessageStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

const DB_MESSAGES_QUEUE: &str = "db.messages";

#[derive(Clone)]
pub struct MessageService {
    repository: Arc<MessageRepository>,
    rabbitmq_con: Arc<Mutex<Connection>>,
}

impl MessageService {
    pub fn new(repository: MessageRepository, rabbitmq_con: Mutex<Connection>) -> Self {
        Self {
            repository: Arc::new(repository),
            rabbitmq_con: Arc::new(rabbitmq_con),
        }
    }
}

impl MessageService {
    pub(super) async fn find_by_participants(
        &self,
        participants: &Vec<String>,
    ) -> Result<Vec<Message>> {
        self.repository.find_by_participants(participants).await
    }
}

impl MessageService {
    /**
     * Publishes a message to a recipient's dedicated queue.
     */
    pub(super) async fn publish_for_recipient(
        &self,
        sender: &str,
        request: MessageRequest,
    ) -> Result<()> {
        self.publish(
            &request.recipient,
            &Message::from_request(sender, request.clone()),
        )
        .await?;
        Ok(())
    }

    /**
     * Publishes a message to a storage queue.
     */
    pub(super) async fn publish_for_storage(&self, data: &str) -> Result<()> {
        let message = serde_json::from_str::<Message>(data).unwrap();
        self.publish(DB_MESSAGES_QUEUE, &message).await?;
        Ok(())
    }

    /**
     * Reads messages from a queue where queue_name is the user's nickname.
     */
    pub(super) async fn read(&self, queue_name: &str) -> Result<(String, Channel, MessageStream)> {
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
                let data = from_utf8(&delivery.data)
                    .expect("not a utf8 string")
                    .to_string();
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
    pub(super) async fn close_consumer(&self, consumer_tag: &str, channel: &Channel) -> Result<()> {
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
            let message_service = self_clone.clone();
            let (consumer_tag, channel, messages_stream) =
                match message_service.read(DB_MESSAGES_QUEUE).await {
                    Ok(binding) => binding,
                    Err(e) => {
                        error!("Failed to read messages: {:?}", e);
                        return;
                    }
                };

            messages_stream
                .for_each(move |data| {
                    let message_repository = self_clone.repository.clone();
                    async move {
                        match data {
                            Ok(data) => {
                                let message = serde_json::from_str::<Message>(&data)
                                    .expect("Failed to deserialize message");
                                if let Err(e) = message_repository.insert(&message).await {
                                    error!("Failed to store message: {:?}", e);
                                }
                            }
                            Err(e) => error!("Failed to read message: {:?}", e),
                        }
                    }
                })
                .await;

            if let Err(e) = message_service
                .close_consumer(&consumer_tag, &channel)
                .await
            {
                error!("Failed to close consumer: {:?}", e);
            };
        });
    }
}

// Private methods
impl MessageService {
    async fn publish(&self, queue_name: &str, payload: &Message) -> Result<()> {
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
