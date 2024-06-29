use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws;
use axum::extract::ws::Message::{Close, Text};
use axum::extract::ws::WebSocket;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use serde_json::from_str;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio::try_join;

use model::Event;
use service::EventService;

use crate::event::error::EventError;
use crate::event::model::{MessagesQueue, Notification};
use crate::user::model::UserInfo;
use crate::user::service::UserService;

pub mod api;
mod context;
pub mod error;
mod model;
pub mod service;

pub type Result<T> = std::result::Result<T, EventError>;

pub(super) async fn handle_socket(
    ws: WebSocket,
    event_service: EventService,
    user_service: UserService,
) {
    let (sender, receiver) = ws.split();
    let ws_ctx = context::Ws::new();

    let read_task = tokio::spawn(read(ws_ctx.clone(), receiver, event_service.clone()));
    let write_task = tokio::spawn(write(
        ws_ctx.clone(),
        sender,
        event_service.clone(),
        user_service.clone(),
    ));

    match try_join!(read_task, write_task) {
        Ok(_) => debug!("ws disconnected gracefully"),
        Err(e) => error!("ws disconnected with error: {}", e),
    }

    remove_online_user(ws_ctx.clone(), user_service).await;
}

pub(super) async fn read(
    ctx: context::Ws,
    mut receiver: SplitStream<WebSocket>,
    event_service: EventService,
) {
    loop {
        tokio::select! {
            // close is notified => stop 'read' task
            _ = ctx.close.notified() => break,

            // read next frame from ws connection
            frame = receiver.next() => {
                if let Some(message) = frame {
                    match message {
                        Err(e) => {
                            error!("failed to read ws frame: {:?}", e);
                            ctx.close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Close(_)) => {
                            debug!("ws connection closed by client");
                            ctx.close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Text(content)) => {
                            if let Err(e) = handle_text_frame(ctx.clone(), content, event_service.clone()).await {
                                error!("failed to handle text frame: {:?}", e);
                                ctx.close.notify_one(); // notify 'write' task to stop
                                break;
                            }
                        },
                        Ok(wtf) => warn!("received non-text ws frame: {:?}", wtf)
                    }
                }
            }
        }
    }
}

pub(super) async fn write(
    ctx: context::Ws,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    user_service: UserService,
) {
    loop {
        // wait for login notification or close
        tokio::select! {
            // close is notified => stop 'write' task
            _ = ctx.close.notified() => return,

            // didn't receive login notification within 5 seconds => stop 'write' task
            _ = sleep(Duration::from_secs(5)) => {
                ctx.close.notify_one(); // notify 'read' task to stop
                return;
            },

            // logged in => break the wait loop and start writing
            _ = ctx.login.notified() => {
                add_online_user(ctx.clone(), user_service.clone()).await;
                break
            },
        }
    }

    let user_info = ctx
        .get_user_info()
        .await
        .expect("user info has to be set when logged in");

    let sub_queue = MessagesQueue::from(user_info.clone().sub);

    let mut notifications_stream = match event_service.read(ctx.clone(), &sub_queue).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to create consumer of notifications: {:?}", e);
            ctx.close.notify_one();
            return;
        }
    };

    let sender = Arc::new(RwLock::new(sender));
    loop {
        tokio::select! {
            // close is notified => stop 'write' task
            _ = ctx.close.notified() => break,

            // push new list of online users every 5 seconds
            _ = sleep(Duration::from_secs(5)) =>
                notify_about_online_users(ctx.clone(), &user_info, user_service.clone(), event_service.clone()).await,

            // new notification is received from queue => send it to the client
            item = notifications_stream.next() => {
                match item {
                    None => break,
                    Some(Err(e)) => error!("Failed to read notification from queue: {:?}", e),
                    Some(Ok(notification)) => send_notification(sender.clone(), &notification).await,
                }
            },
        }
    }

    match event_service.close_channel(ctx).await {
        Ok(_) => debug!("Channel closed"),
        Err(e) => error!("Failed to close channel: {:?}", e),
    }
}

async fn handle_text_frame(
    ctx: context::Ws,
    content: String,
    event_service: EventService,
) -> Result<()> {
    if let Ok(event) = from_str::<Event>(content.as_str()) {
        return event_service.handle_event(ctx.clone(), event).await;
    }
    warn!("skipping frame, content is malformed: {}", content);
    Ok(())
}

async fn send_notification(
    sender: Arc<RwLock<SplitSink<WebSocket, ws::Message>>>,
    notification: &Notification,
) {
    debug!("sending notification: {:?}", notification);
    match serde_json::to_string(&notification) {
        Ok(notification) => {
            let mut sender = sender.write().await;
            if let Err(e) = sender.send(Text(notification)).await {
                error!("Failed to send notification to client: {:?}", e);
            }
        }
        Err(e) => error!("Failed to serialize notification: {:?}", e),
    }
}

async fn notify_about_online_users(
    ctx: context::Ws,
    user_info: &UserInfo,
    user_service: UserService,
    event_service: EventService,
) {
    if let Ok(users) = user_service.get_online_users(user_info.sub.clone()).await {
        if let Err(e) = event_service
            .publish_notification(
                ctx,
                &MessagesQueue::from(user_info.sub.clone()),
                &Notification::UsersOnline { users },
            )
            .await
        {
            error!("failed to publish online users notification: {:?}", e);
        }
    }
}

async fn add_online_user(ctx: context::Ws, user_service: UserService) {
    if let Some(user_info) = ctx.get_user_info().await {
        debug!("adding to online users: {:?}", user_info.sub.clone());
        if let Err(e) = user_service.add_online_user(user_info.sub).await {
            error!("Failed to add user to online users: {:?}", e);
        }
    }
}

async fn remove_online_user(ctx: context::Ws, user_service: UserService) {
    if let Some(user_info) = ctx.get_user_info().await {
        debug!("removing from online users: {:?}", user_info.sub.clone());
        if let Err(e) = user_service.remove_online_user(user_info.sub).await {
            error!("Failed to remove user from online users: {:?}", e);
        }
    }
}
