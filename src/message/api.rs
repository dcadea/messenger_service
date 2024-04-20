use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::{Response, Result};
use axum::routing::get;
use axum::Router;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use tokio::sync::Mutex;

use crate::error::ApiError;
use crate::message::model::MessageRequest;
use crate::message::service::MessageService;
use crate::state::AppState;

pub fn router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws/:recipient", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(recipient): Path<String>,
    state: State<AppState>,
) -> Result<Response, ApiError> {
    if state.user_service.exists(recipient.as_str()).await {
        Ok(ws.on_upgrade(move |socket| {
            handle_socket(socket, recipient, state.message_service.clone())
        }))
    } else {
        Err(ApiError::WebSocketConnectionRejected)
    }
}

async fn handle_socket(ws: WebSocket, recipient: String, message_service: Arc<MessageService>) {
    let (sender, receiver) = ws.split();
    tokio::spawn(read(receiver, message_service.clone()));
    debug!("{} connected", recipient.clone());
    tokio::spawn(write(sender, recipient.clone(), message_service.clone()));
    debug!("{} disconnected", recipient.clone());
}

async fn read(mut receiver: SplitStream<WebSocket>, message_service: Arc<MessageService>) {
    while let Some(next) = receiver.next().await {
        match next {
            Ok(Message::Text(content)) => {
                debug!("received ws message: {:?}", content);
                if let Ok(msg) = serde_json::from_str::<MessageRequest>(content.as_str()) {
                    if let Err(e) = message_service.publish_for_recipient(msg).await {
                        error!("failed to publish message to queue: {:?}, {}", content, e);
                    };
                } else {
                    warn!("skipping message, structure is unsupported: {:?}", content);
                }
                continue;
            }
            Ok(something_else) => {
                warn!("received non-text ws message: {:?}", something_else);
                continue;
            }
            Err(e) => {
                error!("error receiving ws message: {}", e);
                break;
            }
        };
    }
}

async fn write(
    sender: SplitSink<WebSocket, Message>,
    recipient: String,
    message_service: Arc<MessageService>,
) {
    let sender = Arc::new(Mutex::new(sender));

    let (consumer_tag, channel, messages_stream) = match message_service.read(&recipient).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to read messages: {}", e);
            return;
        }
    };

    let message_service_clone = message_service.clone();
    let sender_clone = sender.clone();
    messages_stream
        .for_each(move |data| {
            let message_service = message_service_clone.clone();
            let sender = sender_clone.clone();
            async move {
                match data {
                    Ok(data) => {
                        let message = Message::Text(data.clone());
                        let mut sender = sender.lock().await;
                        let _ = sender.send(message).await;
                        if let Err(e) = message_service.publish_for_storage(data).await {
                            error!("Failed to store message: {}", e);
                        }
                    }
                    Err(e) => error!("Failed to read message: {}", e),
                }
            }
        })
        .await;

    if let Err(e) = message_service.close_consumer(consumer_tag, channel).await {
        error!("Failed to close consumer: {}", e);
    };
}
