use std::io;
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
use tokio::sync::RwLock;
use tokio_stream::Stream;

use crate::auth::service::AuthService;
use crate::chat::service::ChatService;
use crate::error::ApiError;
use crate::message::model::Message;
use crate::message::service::MessageService;
use crate::result::Result;

use super::model::{Event, MessagesQueue, Notification, QueueName, WsCtx};

type NotificationStream = Pin<Box<dyn Stream<Item = Result<Notification>> + Send>>;

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
                    self.auth_service.validate(&token).await?;
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
                Event::CreateMessage {
                    chat_id,
                    recipient,
                    text,
                } => {
                    // TODO: check if owner and recipient are members of the chat
                    let owner = user_info.sub;

                    let message = self
                        .message_service
                        .create(&Message::new(
                            chat_id,
                            owner.clone(),
                            recipient.clone(),
                            &text,
                        ))
                        .await?;

                    let owner_queue: MessagesQueue = owner.into();
                    let recipient_queue: MessagesQueue = recipient.into();
                    let notification = Notification::MessageCreated {
                        message: message.clone(),
                    };

                    tokio::try_join!(
                        self.publish_notification(&owner_queue, &notification),
                        self.publish_notification(&recipient_queue, &notification),
                        self.chat_service.update_last_message(&message)
                    )
                    .map(|_| ())
                }
                Event::UpdateMessage { id, text } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.owner != user_info.sub {
                        return Err(ApiError::Forbidden("You are not the owner".to_owned()));
                    }

                    self.message_service.update(&id, &text).await?;

                    let owner_queue: MessagesQueue = message.owner.into();
                    let recipient_queue: MessagesQueue = message.recipient.into();
                    let notification = Notification::MessageUpdated { id, text };

                    tokio::try_join!(
                        self.publish_notification(&owner_queue, &notification),
                        self.publish_notification(&recipient_queue, &notification)
                    )
                    .map(|_| ())
                }
                Event::DeleteMessage { id } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.owner != user_info.sub {
                        return Err(ApiError::Forbidden("You are not the owner".to_owned()));
                    }
                    self.message_service.delete(&id).await?;

                    let owner_queue: MessagesQueue = message.owner.into();
                    let recipient_queue: MessagesQueue = message.recipient.into();
                    let notification = Notification::MessageDeleted { id };

                    tokio::try_join!(
                        self.publish_notification(&owner_queue, &notification),
                        self.publish_notification(&recipient_queue, &notification)
                    )
                    .map(|_| ())
                }
                Event::SeenMessage { id } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.recipient != user_info.sub {
                        return Err(ApiError::Forbidden("You are not the recipient".to_owned()));
                    }
                    self.message_service.mark_as_seen(&id).await?;

                    let owner_queue: MessagesQueue = message.owner.into();
                    self.publish_notification(&owner_queue, &Notification::MessageSeen { id })
                        .await
                }
            },
        }
    }
}

impl EventService {
    pub async fn read(&self, q: &impl QueueName) -> Result<(String, Channel, NotificationStream)> {
        let (queue_name, channel) = self.split_queue(q).await?;

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
                    let notification = serde_json::from_slice::<Notification>(&data)
                        .map_err(|e| lapin::Error::IOError(Arc::new(io::Error::from(e))))?;
                    delivery.ack(BasicAckOptions::default()).await?;
                    Ok(notification)
                }
            })
            .map_err(ApiError::from);

        Ok((consumer_tag.to_string(), channel, Box::pin(stream)))
    }

    pub async fn close_consumer(&self, consumer_tag: &str, channel: &Channel) -> Result<()> {
        channel
            .basic_cancel(consumer_tag, BasicCancelOptions::default())
            .await?;

        Ok(())
    }
}

impl EventService {
    async fn publish_notification(
        &self,
        q: &impl QueueName,
        notification: &Notification,
    ) -> Result<()> {
        let payload = serde_json::to_vec(notification)?;
        self.publish(q, payload.as_slice()).await
    }

    async fn publish(&self, q: &impl QueueName, payload: &[u8]) -> Result<()> {
        let (queue_name, channel) = self.split_queue(q).await?;

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

    async fn split_queue(&self, q: &impl QueueName) -> Result<(String, Channel)> {
        let conn = self.rabbitmq_con.read().await;
        let channel = conn.create_channel().await?;
        let queue_name = &q.to_string();

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
