use anyhow::Context;
use log::debug;
use tokio_stream::StreamExt;

use crate::integration;
use crate::integration::cache;

use super::model::{Command, Notification, NotificationStream, Queue};

#[derive(Clone)]
pub struct EventService {
    pubsub: async_nats::Client,
    redis: integration::cache::Redis,
}

impl EventService {
    pub fn new(pubsub: async_nats::Client, redis: integration::cache::Redis) -> Self {
        Self { pubsub, redis }
    }
}

impl EventService {
    pub async fn handle_command(&self, command: Command) -> super::Result<()> {
        debug!("handling command: {:?}", command);

        // match command {
        //     Command::CreateMessage {
        //         chat_id,
        //         recipient,
        //         text,
        //     } => {
        //         let owner = ctx.logged_sub.clone();

        //         self.chat_service
        //             .check_members(&chat_id, [&owner.clone(), &recipient.clone()])
        //             .await?;

        //         // TODO: keep in handler or here?
        //         let message = self
        //             .message_service
        //             .create(&Message::new(
        //                 chat_id,
        //                 owner.clone(),
        //                 recipient.clone(),
        //                 &text,
        //             ))
        //             .await?;

        //         let recipient_messages = Queue::Messages(recipient);
        //         let noti = Notification::NewMessage {
        //             message: MessageDto::from(message.clone()),
        //         };

        //         use futures::TryFutureExt;

        //         tokio::try_join!(
        //             self.publish_noti(&recipient_messages, &noti),
        //             self.chat_service
        //                 .update_last_message(&message)
        //                 .map_err(event::Error::from)
        //         )
        //         .map(|_| ())
        //     }
        //     Command::UpdateMessage { id, text } => {
        //         let message = self.message_service.find_by_id(&id).await?;
        //         if message.owner != ctx.logged_sub {
        //             return Err(event::Error::NotOwner);
        //         }

        //         self.message_service.update(&id, &text).await?;

        //         let owner_messages = Queue::Messages(message.owner);
        //         let recipient_messages = Queue::Messages(message.recipient);
        //         let noti = Notification::UpdatedMessage { id, text };

        //         tokio::try_join!(
        //             self.publish_noti(&owner_messages, &noti),
        //             self.publish_noti(&recipient_messages, &noti)
        //         )
        //         .map(|_| ())
        //     }
        //     Command::DeleteMessage(id) => {
        //         let message = self.message_service.find_by_id(&id).await?;
        //         if message.owner != ctx.logged_sub {
        //             return Err(event::Error::NotOwner);
        //         }
        //         self.message_service.delete(&id).await?;

        //         let owner_messages = Queue::Messages(message.owner);
        //         let recipient_messages = Queue::Messages(message.recipient);
        //         let noti = Notification::DeletedMessage { id };

        //         tokio::try_join!(
        //             self.publish_noti(&owner_messages, &noti),
        //             self.publish_noti(&recipient_messages, &noti)
        //         )
        //         .map(|_| ())
        //     }
        //     Command::MarkAsSeen(id) => {
        //         let message = self.message_service.find_by_id(&id).await?;
        //         if message.recipient != ctx.logged_sub {
        //             return Err(event::Error::NotRecipient);
        //         }
        //         self.message_service.mark_as_seen(&id).await?;

        //         let owner_messages = Queue::Messages(message.owner);
        //         self.publish_noti(&owner_messages, &Notification::SeenMessage { id })
        //             .await
        //     }
        // }

        Ok(())
    }
}

impl EventService {
    pub async fn read(&self, q: &Queue) -> super::Result<NotificationStream> {
        let subscriber = self.pubsub.subscribe(q).await?;

        let stream = subscriber.then(|msg| async move {
            match serde_json::from_slice::<Notification>(&msg.payload) {
                Ok(noti) => Some(noti),
                Err(e) => {
                    log::error!("failed to deserialize notification: {:?}", e);
                    None
                }
            }
        });

        Ok(Box::pin(stream))
    }

    pub async fn publish_noti(&self, q: &Queue, noti: &Notification) -> super::Result<()> {
        let payload = serde_json::to_vec(noti)?;
        self.pubsub.publish(q, payload.into()).await?;
        Ok(())
    }
}

impl EventService {
    pub async fn listen_online_status_change(&self) -> anyhow::Result<cache::UpdateStream> {
        let stream = self
            .redis
            .subscribe(&cache::Keyspace::new(cache::Key::UsersOnline))
            .await
            .with_context(|| "Failed to subscribe to online status change")?;

        Ok(stream)
    }
}
