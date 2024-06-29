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

use crate::auth::service::AuthService;
use crate::chat::model::Members;
use crate::chat::service::ChatService;
use crate::message::model::{Message, MessageDto};
use crate::message::service::MessageService;
use crate::user::service::UserService;

use super::error::EventError;
use super::model::{Event, MessagesQueue, Notification, NotificationStream, QueueName};
use super::{context, Result};

#[derive(Clone)]
pub struct EventService {
    amqp_con: Arc<RwLock<Connection>>,
    auth_service: Arc<AuthService>,
    chat_service: Arc<ChatService>,
    message_service: Arc<MessageService>,
    user_service: Arc<UserService>,
}

impl EventService {
    pub fn new(
        amqp_con: RwLock<Connection>,
        auth_service: AuthService,
        chat_service: ChatService,
        message_service: MessageService,
        user_service: UserService,
    ) -> Self {
        Self {
            amqp_con: Arc::new(amqp_con),
            auth_service: Arc::new(auth_service),
            chat_service: Arc::new(chat_service),
            message_service: Arc::new(message_service),
            user_service: Arc::new(user_service),
        }
    }
}

impl EventService {
    pub async fn handle_event(&self, ctx: context::Ws, event: Event) -> Result<()> {
        debug!("handling event: {:?}", event);
        match ctx.get_user_info().await {
            None => {
                if let Event::Auth { token } = event {
                    let claims = self.auth_service.validate(&token).await?;
                    let user_info = self.user_service.find_user_info(claims.sub.clone()).await?;
                    ctx.set_user_info(user_info).await;
                    ctx.set_channel(self.create_channel().await?).await;
                    ctx.login.notify_one();
                    return Ok(());
                }

                Err(EventError::MissingUserInfo)
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
                    let owner = user_info.sub;

                    let members = Members::new(owner.clone(), recipient.clone());
                    self.chat_service.check_members(chat_id, &members).await?;

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
                        message: MessageDto::from(&message),
                    };

                    use futures::TryFutureExt;

                    tokio::try_join!(
                        self.publish_notification(ctx.clone(), &owner_queue, &notification),
                        self.publish_notification(ctx, &recipient_queue, &notification),
                        self.chat_service
                            .update_last_message(&message)
                            .map_err(EventError::from)
                    )
                    .map(|_| ())
                }
                Event::UpdateMessage { id, text } => {
                    let message = self.message_service.find_by_id(id).await?;
                    if message.owner != user_info.sub {
                        return Err(EventError::NotOwner);
                    }

                    self.message_service.update(&id, &text).await?;

                    let owner_queue: MessagesQueue = message.owner.into();
                    let recipient_queue: MessagesQueue = message.recipient.into();
                    let notification = Notification::MessageUpdated { id, text };

                    tokio::try_join!(
                        self.publish_notification(ctx.clone(), &owner_queue, &notification),
                        self.publish_notification(ctx, &recipient_queue, &notification)
                    )
                    .map(|_| ())
                }
                Event::DeleteMessage { id } => {
                    let message = self.message_service.find_by_id(id).await?;
                    if message.owner != user_info.sub {
                        return Err(EventError::NotOwner);
                    }
                    self.message_service.delete(&id).await?;

                    let owner_queue: MessagesQueue = message.owner.into();
                    let recipient_queue: MessagesQueue = message.recipient.into();
                    let notification = Notification::MessageDeleted { id };

                    tokio::try_join!(
                        self.publish_notification(ctx.clone(), &owner_queue, &notification),
                        self.publish_notification(ctx, &recipient_queue, &notification)
                    )
                    .map(|_| ())
                }
                Event::SeenMessage { id } => {
                    let message = self.message_service.find_by_id(id).await?;
                    if message.recipient != user_info.sub {
                        return Err(EventError::NotRecipient);
                    }
                    self.message_service.mark_as_seen(&id).await?;

                    let owner_queue: MessagesQueue = message.owner.into();
                    self.publish_notification(ctx, &owner_queue, &Notification::MessageSeen { id })
                        .await
                }
            },
        }
    }
}

impl EventService {
    pub async fn read(&self, ctx: context::Ws, q: &impl QueueName) -> Result<NotificationStream> {
        self.ensure_queue_exists(ctx.clone(), q).await?;

        let consumer = ctx
            .get_channel()
            .await?
            .basic_consume(
                &q.to_string(),
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

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
            .map_err(EventError::from);

        Ok(Box::pin(stream))
    }

    pub async fn close_channel(&self, ctx: context::Ws) -> Result<()> {
        let channel = ctx.get_channel().await?;
        channel.close(200, "OK").await.map_err(EventError::from)
    }

    pub async fn publish_notification(
        &self,
        ctx: context::Ws,
        q: &impl QueueName,
        notification: &Notification,
    ) -> Result<()> {
        let payload = serde_json::to_vec(notification)?;
        self.publish(ctx, q, payload.as_slice()).await
    }
}

impl EventService {
    async fn create_channel(&self) -> Result<Channel> {
        let conn = self.amqp_con.read().await;
        conn.create_channel().await.map_err(EventError::from)
    }

    async fn publish(&self, ctx: context::Ws, q: &impl QueueName, payload: &[u8]) -> Result<()> {
        self.ensure_queue_exists(ctx.clone(), q).await?;
        ctx.get_channel()
            .await?
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

    async fn ensure_queue_exists(&self, ctx: context::Ws, q: &impl QueueName) -> Result<()> {
        ctx.get_channel()
            .await?
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
            .map_err(EventError::from)
    }
}
