use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::{Response, Result};
use axum::routing::get;
use axum::Router;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use tokio::sync::{Mutex, Notify};
use tokio::try_join;

use crate::error::ApiError;
use crate::message::model::MessageRequest;
use crate::message::service::MessageService;
use crate::state::AppState;

pub fn ws_router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws/:topic", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(topic): Path<String>,
    state: State<AppState>,
) -> Result<Response, ApiError> {
    if !state.user_service.exists(topic.as_str()).await {
        return Err(ApiError::WebSocketConnectionRejected);
    }

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, topic, state.message_service.clone())))
}

async fn handle_socket(ws: WebSocket, topic: String, ms: Arc<MessageService>) {
    let (sender, receiver) = ws.split();
    let notify = Arc::new(Notify::new());
    let read_task = tokio::spawn(read(receiver, ms.clone(), notify.clone()));
    let write_task = tokio::spawn(write(topic.clone(), sender, ms.clone(), notify.clone()));
    debug!("'{}' connected", topic.clone());

    match try_join!(read_task, write_task) {
        Ok(_) => debug!("'{:?}' disconnected gracefully", topic.clone()),
        Err(e) => error!("'{:?}' disconnected with error: {}", topic.clone(), e),
    }
}

async fn read(mut receiver: SplitStream<WebSocket>, ms: Arc<MessageService>, notify: Arc<Notify>) {
    while let Some(frame) = receiver.next().await {
        match frame {
            Ok(Message::Text(content)) => {
                debug!("received ws frame: {:?}", content);
                if let Ok(msg) = serde_json::from_str::<MessageRequest>(content.as_str()) {
                    if let Err(e) = ms.publish_for_recipient(msg).await {
                        error!("failed to publish message to queue: {}, {:?}", content, e);
                    };
                } else {
                    warn!("skipping frame, content is malformed: {}", content);
                }
            }
            Ok(Message::Close(_)) => {
                notify.notify_one();
                break;
            }
            Ok(wtf) => debug!("received non-text ws frame: {:?}", wtf),
            Err(e) => {
                error!("error receiving ws frame: {:?}", e);
                break;
            }
        };
    }
}

async fn write(
    topic: String,
    sender: SplitSink<WebSocket, Message>,
    ms: Arc<MessageService>,
    notify: Arc<Notify>,
) {
    let sender = Arc::new(Mutex::new(sender));

    let (consumer_tag, channel, mut messages_stream) = match ms.read(&topic).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to read messages: {:?}", e);
            return;
        }
    };

    loop {
        tokio::select! {
            item = messages_stream.next() => {
                match item {
                    Some(Ok(item)) => {
                        let message = Message::Text(item.clone());
                        let mut sender = sender.lock().await;
                        let _ = sender.send(message).await;
                        if let Err(e) = ms.publish_for_storage(item).await {
                            error!("Failed to store message: {:?}", e);
                        }
                    },
                    Some(Err(e)) => error!("Failed to read message: {:?}", e),
                    None => break,
                }
            },
            _ = notify.notified() => break,
        }
    }

    match ms.close_consumer(consumer_tag.clone(), channel).await {
        Ok(_) => debug!("Consumer '{:?}' closed", consumer_tag),
        Err(e) => error!("Failed to close consumer: {:?}", e),
    };
}
