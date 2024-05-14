use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::Message::{Binary, Close, Text};
use axum::extract::ws::WebSocket;
use axum::extract::{ws, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use serde_json::from_str;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio::try_join;

use crate::event::model::{Event, WsContext};
use crate::event::service::EventService;
use crate::message::service::MessageService;
use crate::result::Result;
use crate::state::AppState;

pub fn ws_router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(event_service): State<EventService>,
    State(message_service): State<MessageService>,
) -> Result<Response> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, event_service, message_service)))
}

async fn handle_socket(
    ws: WebSocket,
    event_service: EventService,
    message_service: MessageService,
) {
    let (sender, receiver) = ws.split();
    let context = WsContext::new();

    let read_task = tokio::spawn(read(context.clone(), receiver, event_service.clone()));
    let write_task = tokio::spawn(write(
        context.clone(),
        sender,
        event_service.clone(),
        message_service.clone(),
    ));

    match try_join!(read_task, write_task) {
        Ok(_) => debug!("ws disconnected gracefully"),
        Err(e) => error!("ws disconnected with error: {}", e),
    }
}

async fn read(
    context: WsContext,
    mut receiver: SplitStream<WebSocket>,
    event_service: EventService,
) {
    loop {
        tokio::select! {
            _ = context.close.notified() => break,
            frame = receiver.next() => {
                if let Some(message) = frame {
                    match message {
                        Err(e) => {
                            error!("failed to read ws frame: {:?}", e);
                            context.close.notify_one();
                            break;
                        },
                        Ok(Close(_)) => {
                            debug!("ws connection closed by client");
                            context.close.notify_one();
                            break;
                        },
                        Ok(Text(content)) => {
                            if let Ok(event) = from_str::<Event>(content.as_str()) {
                                if let Err(e) = event_service.handle_event(context.clone(), event).await {
                                    error!("failed to handle event: {:?}", e);
                                    context.close.notify_one();
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

async fn write(
    context: WsContext,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    message_service: MessageService,
) {
    loop {
        tokio::select! {
            _ = context.login.notified() => break,
            _ = context.close.notified() => return,
            _ = sleep(Duration::from_secs(5)) => {
                context.close.notify_one();
                return;
            },
        }
    }

    let nickname = &context
        .get_user_info()
        .await
        .expect("not authorized user")
        .nickname;

    let (consumer_tag, channel, mut messages_stream) = match event_service.read(nickname).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to read messages: {:?}", e);
            context.close.notify_one();
            return;
        }
    };

    let sender = Arc::new(RwLock::new(sender));
    loop {
        tokio::select! {
            item = messages_stream.next() => {
                match item {
                    Some(Ok(message_id)) => {
                        let mut sender = sender.write().await;
                        match message_service.find_by_id(&message_id).await {
                            Some(message) => {
                                let message = serde_json::to_vec(&message).expect("failed to serialize message");
                                if let Err(e) = sender.send(Binary(message)).await {
                                    error!("Failed to send message: {:?}", e);
                                }
                            },
                            None => warn!("Message not found: {:?}", message_id),
                        }
                    },
                    Some(Err(e)) => error!("Failed to read message: {:?}", e),
                    None => break,
                }
            },
            _ = context.close.notified() => break,
        }
    }

    match event_service.close_consumer(&consumer_tag, &channel).await {
        Ok(_) => debug!("Consumer {:?} closed", consumer_tag),
        Err(e) => error!("Failed to close consumer: {:?}", e),
    };
}
