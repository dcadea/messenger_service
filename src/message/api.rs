use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::{Response, Result};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{FutureExt, StreamExt};
use log::{debug, error};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::error::ApiError;
use crate::message::model::{MessageRequest, MessageResponse};
use crate::message::service::MessageService;
use crate::state::AppState;

pub fn router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws/:recipient", get(ws_handler))
        .route("/messages", post(messages_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(recipient): Path<String>,
    state: State<AppState>,
) -> Result<Response, ApiError> {
    if state.user_service.exists(recipient.as_str()).await {
        Ok(ws.on_upgrade(move |socket| {
            client_connection(socket, recipient, state.message_service.clone())
        }))
    } else {
        Err(ApiError::WebSocketConnectionRejected)
    }
}

async fn messages_handler(
    state: State<AppState>,
    Json(request): Json<MessageRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    state
        .message_service
        .publish_for_recipient(request)
        .await
        .map(|v| Ok(Json(v)))?
}

async fn client_connection(ws: WebSocket, recipient: String, message_service: Arc<MessageService>) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_sender = Arc::new(Mutex::new(client_sender));

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("error sending websocket msg: {}", e);
        }
    }));

    debug!("{} connected", recipient);

    let (consumer_tag, channel, messages_stream) = match message_service.read(&recipient).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to read messages: {}", e);
            return;
        }
    };

    let client_sender_clone = client_sender.clone();
    let message_service_clone = message_service.clone();
    tokio::spawn(messages_stream.for_each(move |data| {
        let client_sender = client_sender_clone.clone();
        let message_service = message_service_clone.clone();
        async move {
            match data {
                Ok(data) => {
                    let _ = client_sender
                        .lock()
                        .await
                        .send(Ok(Message::Text(data.clone())));
                    if let Err(e) = message_service.publish_for_storage(data).await {
                        error!("Failed to store message: {}", e);
                    }
                }
                Err(e) => error!("Failed to read message: {}", e),
            }
        }
    }));

    while let Some(result) = client_ws_rcv.next().await {
        let _ = match result {
            Ok(_) => continue,
            Err(e) => {
                error!(
                    "error receiving ws message for recipient: {}): {}",
                    recipient.clone(),
                    e
                );
                break;
            }
        };
    }

    if let Err(e) = message_service.close_consumer(consumer_tag, channel).await {
        error!("Failed to close consumer: {}", e);
    };

    debug!("{} disconnected", recipient);
}
