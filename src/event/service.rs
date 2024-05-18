use std::pin::Pin;
use std::sync::Arc;

use futures::TryStreamExt;
use lapin::options::{
    BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use log::{debug, error};
use mongodb::bson::oid::ObjectId;
use tokio::sync::RwLock;
use tokio_stream::Stream;

use super::model::{Event, MessageQueue, QueueName, WsCtx};
use crate::auth::service::AuthService;
use crate::chat::service::ChatService;
use crate::error::ApiError;
use crate::message::model::{Message, MessageId};
use crate::message::service::MessageService;
use crate::result::Result;

type MessageIdStream = Pin<Box<dyn Stream<Item = Result<MessageId>> + Send>>;

#[derive(Clone)]
pub struct EventService {
    rabbitmq_con: Arc<RwLock<Connection>>,
    chat_service: Arc<ChatService>,
    message_service: Arc<MessageService>,
    auth_service: Arc<AuthService>,
}

impl EventService {
    pub fn new(
        rabbitmq_con: RwLock<Connection>,
        chat_service: ChatService,
        message_service: MessageService,
        auth_service: AuthService,
    ) -> Self {
        Self {
            rabbitmq_con: Arc::new(rabbitmq_con),
            chat_service: Arc::new(chat_service),
            message_service: Arc::new(message_service),
            auth_service: Arc::new(auth_service),
        }
    }
}

impl EventService {
    pub async fn handle_event(&self, ctx: WsCtx, event: Event) -> Result<()> {
        debug!("handling event: {:?}", event);
        match ctx.get_user_info().await {
            None => {
                if let Event::Auth { token } = event {
                    let _ = self.auth_service.validate(&token).await?;
                    let user_info = self.auth_service.get_user_info(&token).await?;
                    ctx.set_user_info(user_info).await;
                    ctx.login.notify_one();
                    return Ok(());
                }

                error!("user info is not set");
                Err(ApiError::Unauthorized)
            }
            Some(user_info) => match event {
                Event::Auth { .. } => {
                    debug!("received auth request with user info already set, ignoring");
                    Ok(())
                }
                Event::CreateMessage { recipient, text } => {
                    let sender = user_info.sub.as_str();
                    let message_id = self
                        .message_service
                        .create(&Message::new(&sender, &recipient, &text))
                        .await?;

                    self.publish_message_id(&MessageQueue::new(sender), &message_id)
                        .await?;
                    self.publish_message_id(&MessageQueue::new(&recipient), &message_id)
                        .await?;
                    self.chat_service
                        .update_last_message(&sender, &recipient, &text)
                        .await
                }
                Event::UpdateMessage { id, text } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.sender != user_info.sub {
                        return Err(ApiError::Forbidden("You are not the sender".to_owned()));
                    }
                    self.message_service.update(&id, &text).await
                }
                Event::DeleteMessage { id } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.sender != user_info.sub {
                        return Err(ApiError::Forbidden("You are not the sender".to_owned()));
                    }
                    self.message_service.delete(&id).await
                }
                Event::SeenMessage { id } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.recipient != user_info.sub {
                        return Err(ApiError::Forbidden("You are not the recipient".to_owned()));
                    }
                    self.message_service.mark_as_seen(&id).await?;
                    self.publish_message_id(&MessageQueue::new(&message.sender), &id)
                        .await
                }
            },
        }
    }
}

impl EventService {
    pub async fn publish_message_id(&self, q: &impl QueueName, id: &MessageId) -> Result<()> {
        self.publish(&q.to_string(), &id.bytes()).await
    }

    pub async fn read(&self, q: &impl QueueName) -> Result<(String, Channel, MessageIdStream)> {
        let (queue_name, channel) = self.split_queue(&q.to_string()).await?;

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
                    let vec: [u8; 12] = data.try_into().expect("Wrong length for ObjectId");
                    Ok(ObjectId::from_bytes(vec))
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
        let conn = self.rabbitmq_con.read().await;
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
