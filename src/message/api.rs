use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Extension, Json, Router};
use axum_extra::extract::Query;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use tokio::sync::{Mutex, Notify};
use tokio::try_join;

use crate::error::ApiError;
use crate::message::model::{Message, MessageParams, MessageRequest};
use crate::message::service::MessageService;
use crate::result::Result;
use crate::state::AppState;
use crate::user::model::User;

pub fn resources<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/messages", get(find_handler))
        .with_state(state)
}

pub fn ws_router<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn find_handler(
    Query(params): Query<MessageParams>,
    Extension(user): Extension<User>,
    state: State<AppState>,
) -> Result<Json<Vec<Message>>> {
    match params.recipient {
        None => Err(ApiError::QueryParamRequired("recipient".to_owned())),
        Some(recipient) => {
            let mut participants = recipient.clone();
            participants.push(user.nickname);

            state
                .message_service
                .find_by_participants(&participants)
                .await
                .map(Json)
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(user): Extension<User>,
    state: State<AppState>,
) -> Result<Response> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, user, state.message_service.clone())))
}

async fn handle_socket(ws: WebSocket, user: User, message_service: Arc<MessageService>) {
    let nickname = user.nickname.clone(); // also a topic name
    let (sender, receiver) = ws.split();
    let notify = Arc::new(Notify::new());

    let read_task = tokio::spawn(read(
        nickname.clone(),
        receiver,
        message_service.clone(),
        notify.clone(),
    ));

    let write_task = tokio::spawn(write(
        nickname.clone(),
        sender,
        message_service.clone(),
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
    message_service: Arc<MessageService>,
    notify: Arc<Notify>,
) {
    while let Some(frame) = receiver.next().await {
        match frame {
            Ok(WsMessage::Text(content)) => {
                debug!("received ws frame: {:?}", content);
                if content.trim() == "ping" {
                    continue;
                }

                if let Ok(msg) = serde_json::from_str::<MessageRequest>(content.as_str()) {
                    if let Err(e) = message_service.publish_for_recipient(&nickname, msg).await {
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
    message_service: Arc<MessageService>,
    notify: Arc<Notify>,
) {
    let sender = Arc::new(Mutex::new(sender));

    let (consumer_tag, channel, mut messages_stream) = match message_service.read(&nickname).await {
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
                        let message = WsMessage::Text(item.clone());
                        let mut sender = sender.lock().await;
                        let _ = sender.send(message).await;
                        if let Err(e) = message_service.publish_for_storage(&item).await {
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

    match message_service
        .close_consumer(&consumer_tag, &channel)
        .await
    {
        Ok(_) => debug!("Consumer '{:?}' closed", consumer_tag),
        Err(e) => error!("Failed to close consumer: {:?}", e),
    };
}
