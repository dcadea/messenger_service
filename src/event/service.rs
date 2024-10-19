use std::io;
use std::sync::Arc;

use futures::TryStreamExt;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use log::debug;
use tokio::sync::RwLock;

use crate::chat::service::ChatService;
use crate::event;
use crate::message::model::{Message, MessageDto};
use crate::message::service::MessageService;

use super::context;
use super::model::{Command, Notification, NotificationStream, Queue};

#[derive(Clone)]
pub struct EventService {
    amqp_con: Arc<RwLock<Connection>>,
    chat_service: Arc<ChatService>,
    message_service: Arc<MessageService>,
}

impl EventService {
    pub fn new(
        amqp_con: RwLock<Connection>,
        chat_service: ChatService,
        message_service: MessageService,
    ) -> Self {
        Self {
            amqp_con: Arc::new(amqp_con),
            chat_service: Arc::new(chat_service),
            message_service: Arc::new(message_service),
        }
    }
}

impl EventService {
    pub async fn handle_command(&self, ctx: &context::Ws, command: Command) -> super::Result<()> {
        debug!("handling command: {:?}", command);

        let logged_sub = ctx.user_info.sub.clone();

        match command {
            Command::CreateMessage {
                chat_id,
                recipient,
                text,
            } => {
                let owner = logged_sub.clone();

                self.chat_service
                    .check_members(&chat_id, [&owner.clone(), &recipient.clone()])
                    .await?;

                let message = self
                    .message_service
                    .create(&Message::new(
                        chat_id,
                        owner.clone(),
                        recipient.clone(),
                        &text,
                    ))
                    .await?;

                let owner_messages = Queue::Messages(owner);
                let recipient_messages = Queue::Messages(recipient);
                let noti = Notification::NewMessage {
                    message: MessageDto::from(message.clone()),
                };

                use futures::TryFutureExt;

                tokio::try_join!(
                    self.publish_noti(ctx, &owner_messages, &noti),
                    self.publish_noti(ctx, &recipient_messages, &noti),
                    self.chat_service
                        .update_last_message(&message)
                        .map_err(event::Error::from)
                )
                .map(|_| ())
            }
            Command::UpdateMessage { id, text } => {
                let message = self.message_service.find_by_id(&id).await?;
                if message.owner != logged_sub {
                    return Err(event::Error::NotOwner);
                }

                self.message_service.update(&id, &text).await?;

                let owner_messages = Queue::Messages(message.owner);
                let recipient_messages = Queue::Messages(message.recipient);
                let noti = Notification::UpdatedMessage { id, text };

                tokio::try_join!(
                    self.publish_noti(ctx, &owner_messages, &noti),
                    self.publish_noti(ctx, &recipient_messages, &noti)
                )
                .map(|_| ())
            }
            Command::DeleteMessage(id) => {
                let message = self.message_service.find_by_id(&id).await?;
                if message.owner != logged_sub {
                    return Err(event::Error::NotOwner);
                }
                self.message_service.delete(&id).await?;

                let owner_messages = Queue::Messages(message.owner);
                let recipient_messages = Queue::Messages(message.recipient);
                let noti = Notification::DeletedMessage { id };

                tokio::try_join!(
                    self.publish_noti(ctx, &owner_messages, &noti),
                    self.publish_noti(ctx, &recipient_messages, &noti)
                )
                .map(|_| ())
            }
            Command::MarkAsSeen(id) => {
                let message = self.message_service.find_by_id(&id).await?;
                if message.recipient != logged_sub {
                    return Err(event::Error::NotRecipient);
                }
                self.message_service.mark_as_seen(&id).await?;

                let owner_messages = Queue::Messages(message.owner);
                self.publish_noti(ctx, &owner_messages, &Notification::SeenMessage { id })
                    .await
            }
        }
    }
}

impl EventService {
    pub async fn read(&self, ctx: &context::Ws, q: &Queue) -> super::Result<NotificationStream> {
        self.ensure_queue_exists(ctx, q).await?;

        let consumer = ctx
            .get_channel()
            .await
            .basic_consume(
                &q.to_string(),
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let stream = consumer
            .and_then(|delivery| async move {
                let noti = serde_json::from_slice::<Notification>(&delivery.data)
                    .map_err(|e| lapin::Error::IOError(Arc::new(io::Error::from(e))))?;
                delivery.ack(BasicAckOptions::default()).await?;
                Ok(noti)
            })
            .map_err(event::Error::from);

        Ok(Box::pin(stream))
    }

    pub async fn close_channel(&self, ctx: &context::Ws) -> super::Result<()> {
        let channel = ctx.get_channel().await;
        channel.close(200, "OK").await.map_err(event::Error::from)
    }

    pub async fn publish_noti(
        &self,
        ctx: &context::Ws,
        q: &Queue,
        noti: &Notification,
    ) -> super::Result<()> {
        let payload = serde_json::to_vec(noti)?;
        self.publish(ctx, q, payload.as_slice()).await
    }
}

impl EventService {
    pub async fn create_channel(&self) -> super::Result<Channel> {
        let conn = self.amqp_con.read().await;
        conn.create_channel().await.map_err(event::Error::from)
    }

    async fn publish(&self, ctx: &context::Ws, q: &Queue, payload: &[u8]) -> super::Result<()> {
        self.ensure_queue_exists(ctx, q).await?;
        ctx.get_channel()
            .await
            .basic_publish(
                "",
                &q.to_string(),
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default(),
            )
            .await?;
        Ok(())
    }

    async fn ensure_queue_exists(&self, ctx: &context::Ws, q: &Queue) -> super::Result<()> {
        ctx.get_channel()
            .await
            .queue_declare(
                &q.to_string(),
                QueueDeclareOptions {
                    auto_delete: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await
            .map(|_| ())
            .map_err(event::Error::from)
    }
}
