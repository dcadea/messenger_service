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
use crate::chat::service::ChatService;
use crate::event;
use crate::message::model::{Message, MessageDto};
use crate::message::service::MessageService;
use crate::user::service::UserService;

use super::context;
use super::model::{Command, Event, EventStream, Queue};

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
    pub async fn handle_command(&self, ctx: &context::Ws, command: Command) -> super::Result<()> {
        debug!("handling command: {:?}", command);
        match ctx.get_user_info().await {
            None => {
                if let Command::Auth { token } = command {
                    let sub = self.auth_service.validate(&token).await?;
                    let user_info = self.user_service.find_user_info(&sub).await?;
                    ctx.set_user_info(user_info).await;
                    ctx.set_channel(self.create_channel().await?).await;
                    ctx.login.notify_one();
                    return Ok(());
                }

                Err(event::Error::MissingUserInfo)
            }
            Some(user_info) => match command {
                Command::Auth { .. } => {
                    debug!("received auth request with user info already set, ignoring");
                    Ok(())
                }
                Command::CreateMessage {
                    chat_id,
                    recipient,
                    text,
                } => {
                    let owner = user_info.sub;

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
                    let event = Event::NewMessage {
                        message: MessageDto::from(message.clone()),
                    };

                    use futures::TryFutureExt;

                    tokio::try_join!(
                        self.publish_event(ctx, &owner_messages, &event),
                        self.publish_event(ctx, &recipient_messages, &event),
                        self.chat_service
                            .update_last_message(&message)
                            .map_err(event::Error::from)
                    )
                    .map(|_| ())
                }
                Command::UpdateMessage { id, text } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.owner != user_info.sub {
                        return Err(event::Error::NotOwner);
                    }

                    self.message_service.update(&id, &text).await?;

                    let owner_messages = Queue::Messages(message.owner);
                    let recipient_messages = Queue::Messages(message.recipient);
                    let event = Event::UpdatedMessage { id, text };

                    tokio::try_join!(
                        self.publish_event(ctx, &owner_messages, &event),
                        self.publish_event(ctx, &recipient_messages, &event)
                    )
                    .map(|_| ())
                }
                Command::DeleteMessage { id } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.owner != user_info.sub {
                        return Err(event::Error::NotOwner);
                    }
                    self.message_service.delete(&id).await?;

                    let owner_messages = Queue::Messages(message.owner);
                    let recipient_messages = Queue::Messages(message.recipient);
                    let event = Event::DeletedMessage { id };

                    tokio::try_join!(
                        self.publish_event(ctx, &owner_messages, &event),
                        self.publish_event(ctx, &recipient_messages, &event)
                    )
                    .map(|_| ())
                }
                Command::MarkAsSeenMessage { id } => {
                    let message = self.message_service.find_by_id(&id).await?;
                    if message.recipient != user_info.sub {
                        return Err(event::Error::NotRecipient);
                    }
                    self.message_service.mark_as_seen(&id).await?;

                    let owner_messages = Queue::Messages(message.owner);
                    self.publish_event(ctx, &owner_messages, &Event::SeenMessage { id })
                        .await
                }
            },
        }
    }
}

impl EventService {
    pub async fn read(&self, ctx: &context::Ws, q: &Queue) -> super::Result<EventStream> {
        self.ensure_queue_exists(ctx, q).await?;

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
            .and_then(|delivery| async move {
                let event = serde_json::from_slice::<Event>(&delivery.data)
                    .map_err(|e| lapin::Error::IOError(Arc::new(io::Error::from(e))))?;
                delivery.ack(BasicAckOptions::default()).await?;
                Ok(event)
            })
            .map_err(event::Error::from);

        Ok(Box::pin(stream))
    }

    pub async fn close_channel(&self, ctx: &context::Ws) -> super::Result<()> {
        let channel = ctx.get_channel().await?;
        channel.close(200, "OK").await.map_err(event::Error::from)
    }

    pub async fn publish_event(
        &self,
        ctx: &context::Ws,
        q: &Queue,
        event: &Event,
    ) -> super::Result<()> {
        let payload = serde_json::to_vec(event)?;
        self.publish(ctx, q, payload.as_slice()).await
    }
}

impl EventService {
    async fn create_channel(&self) -> super::Result<Channel> {
        let conn = self.amqp_con.read().await;
        conn.create_channel().await.map_err(event::Error::from)
    }

    async fn publish(&self, ctx: &context::Ws, q: &Queue, payload: &[u8]) -> super::Result<()> {
        self.ensure_queue_exists(ctx, q).await?;
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

    async fn ensure_queue_exists(&self, ctx: &context::Ws, q: &Queue) -> super::Result<()> {
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
            .map_err(event::Error::from)
    }
}
