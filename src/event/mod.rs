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

use crate::event::error::EventError;

use crate::event::model::MessagesQueue;
use model::{Event, WsCtx};
use service::EventService;

pub mod api;
pub mod error;
mod model;
pub mod service;

pub type Result<T> = std::result::Result<T, EventError>;

pub(super) async fn handle_socket(ws: WebSocket, event_service: EventService) {
    let (sender, receiver) = ws.split();
    let ctx = WsCtx::new();

    let read_task = tokio::spawn(read(ctx.clone(), receiver, event_service.clone()));
    let write_task = tokio::spawn(write(ctx.clone(), sender, event_service.clone()));

    match try_join!(read_task, write_task) {
        Ok(_) => debug!("ws disconnected gracefully"),
        Err(e) => error!("ws disconnected with error: {}", e),
    }
}

pub(super) async fn read(
    ctx: WsCtx,
    mut receiver: SplitStream<WebSocket>,
    event_service: EventService,
) {
    loop {
        tokio::select! {
            _ = ctx.close.notified() => break,
            frame = receiver.next() => {
                if let Some(message) = frame {
                    match message {
                        Err(e) => {
                            error!("failed to read ws frame: {:?}", e);
                            ctx.close.notify_one();
                            break;
                        },
                        Ok(Close(_)) => {
                            debug!("ws connection closed by client");
                            ctx.close.notify_one();
                            break;
                        },
                        Ok(Text(content)) => {
                            if let Ok(event) = from_str::<Event>(content.as_str()) {
                                if let Err(e) = event_service.handle_event(ctx.clone(), event).await {
                                    error!("failed to handle event: {:?}", e);
                                    ctx.close.notify_one();
                                    break;
                                }
                            } else {
                                warn!("skipping frame, content is malformed: {}", content);
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
    ctx: WsCtx,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
) {
    loop {
        tokio::select! {
            _ = ctx.login.notified() => break,
            _ = ctx.close.notified() => return,
            _ = sleep(Duration::from_secs(5)) => {
                ctx.close.notify_one();
                return;
            },
        }
    }

    match ctx.get_user_info().await {
        None => {
            error!("not authorized user");
            ctx.close.notify_one();
            return;
        }
        Some(user_info) => {
            let sub_queue: MessagesQueue = user_info.sub.into();

            let (consumer_tag, channel, mut notifications_stream) =
                match event_service.read(&sub_queue).await {
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
                    item = notifications_stream.next() => {
                        match item {
                            None => break,
                            Some(Err(e)) => error!("Failed to read notification from queue: {:?}", e),
                            Some(Ok(notification)) => {
                                debug!("sending notification: {:?}", notification);
                                let mut sender = sender.write().await;
                                match serde_json::to_string(&notification) {
                                    Ok(notification) => {
                                        if let Err(e) = sender.send(Text(notification)).await {
                                            error!("Failed to send notification to client: {:?}", e);
                                        }
                                    }
                                    Err(e) => error!("Failed to serialize notification: {:?}", e),
                                }
                            },
                        }
                    },
                    _ = ctx.close.notified() => break,
                }
            }

            match event_service.close_consumer(&consumer_tag, &channel).await {
                Ok(_) => debug!("Consumer {:?} closed", consumer_tag),
                Err(e) => error!("Failed to close consumer: {:?}", e),
            }
        }
    }
}
