use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use tokio::sync::{Mutex, Notify};
use tokio::try_join;

use crate::event::service::EventService;
use crate::message::model::MessageRequest;
use crate::result::Result;
use crate::state::AppState;

pub fn ws_router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws/:topic", get(ws_handler)) // FIXME: remove topic from path
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(topic): Path<String>, // FIXME: define a ws session context and store user info there
    State(event_service): State<EventService>,
) -> Result<Response> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, topic, event_service.clone())))
}

async fn handle_socket(ws: WebSocket, topic: String, event_service: EventService) {
    let nickname = topic.clone(); // also a topic name
    let (sender, receiver) = ws.split();
    let notify = Arc::new(Notify::new());

    let read_task = tokio::spawn(read(
        nickname.clone(),
        receiver,
        event_service.clone(),
        notify.clone(),
    ));

    let write_task = tokio::spawn(write(
        nickname.clone(),
        sender,
        event_service.clone(),
        notify.clone(),
    ));

    debug!("'{}' connected", nickname.clone());

    match try_join!(read_task, write_task) {
        Ok(_) => debug!("'{:?}' disconnected gracefully", nickname.clone()),
        Err(e) => error!("'{:?}' disconnected with error: {}", nickname.clone(), e),
    }
}

async fn read(
    nickname: String,
    mut receiver: SplitStream<WebSocket>,
    event_service: EventService,
    notify: Arc<Notify>,
) {
    while let Some(frame) = receiver.next().await {
        match frame {
            Ok(WsMessage::Text(content)) => {
                debug!("received ws frame: {:?}", content);

                if let Ok(message_request) =
                    serde_json::from_str::<MessageRequest>(content.as_str())
                {
                    if let Err(e) = event_service
                        .publish_for_recipient(&nickname, &message_request)
                        .await
                    {
                        error!("failed to publish message to queue: {}, {:?}", content, e);
                    };
                } else {
                    warn!("skipping frame, content is malformed: {}", content);
                }
            }
            Ok(WsMessage::Close(_)) => {
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
    nickname: String,
    sender: SplitSink<WebSocket, WsMessage>,
    event_service: EventService,
    notify: Arc<Notify>,
) {
    let sender = Arc::new(Mutex::new(sender));

    let (consumer_tag, channel, mut messages_stream) = match event_service.read(&nickname).await {
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
                        let message = WsMessage::Binary(item.clone());
                        let mut sender = sender.lock().await;
                        let _ = sender.send(message).await;
                        if let Err(e) = event_service.publish_for_storage(item.as_slice()).await {
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

    match event_service.close_consumer(&consumer_tag, &channel).await {
        Ok(_) => debug!("Consumer '{:?}' closed", consumer_tag),
        Err(e) => error!("Failed to close consumer: {:?}", e),
    };
}