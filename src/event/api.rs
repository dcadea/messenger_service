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

use crate::auth::service::AuthService;
use crate::event::model::{EventRequest, WsRequestContext};
use crate::event::service::EventService;
use crate::message::model::MessageRequest;
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
    State(auth_service): State<AuthService>,
) -> Result<Response> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, event_service, auth_service)))
}

async fn handle_socket(ws: WebSocket, event_service: EventService, auth_service: AuthService) {
    let (sender, receiver) = ws.split();
    let context = WsRequestContext::new();

    let read_task = tokio::spawn(read(
        receiver,
        event_service.clone(),
        auth_service.clone(),
        context.clone(),
    ));

    let write_task = tokio::spawn(write(sender, event_service.clone(), context.clone()));

    match try_join!(read_task, write_task) {
        Ok(_) => debug!("ws disconnected gracefully"),
        Err(e) => error!("ws disconnected with error: {}", e),
    }
}

async fn read(
    mut receiver: SplitStream<WebSocket>,
    event_service: EventService,
    auth_service: AuthService,
    context: WsRequestContext,
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
                            if let Ok(event_request) = from_str::<EventRequest>(content.as_str()) {
                                match context.get_user_info().await {
                                    None => {
                                        debug!("no user info, expecting auth request");
                                        if let EventRequest::Auth { token } = event_request {
                                            debug!("received auth request: {}", token);
                                            match auth_service.validate(&token).await {
                                                Ok(_) => {
                                                    match auth_service.get_user_info(&token).await {
                                                        Ok(user_info) => {
                                                            context.set_user_info(user_info).await;
                                                            context.login.notify_one();
                                                            continue;
                                                        },
                                                        Err(e) => error!("failed to get user info: {:?}", e),
                                                    }
                                                },
                                                Err(e) => error!("failed to validate token: {:?}", e),
                                            }
                                        }
                                        error!("initial request is not auth, closing connection");
                                        context.close.notify_one();
                                        break;
                                    }
                                    Some(user_info) => {
                                        if let EventRequest::CreateMessage { recipient, text } = event_request {
                                            debug!("received message request: {:?} -> {}", recipient, text);
                                            let nickname = user_info.nickname.clone();
                                            let message_request = MessageRequest { recipient, text };
                                            if let Err(e) = event_service
                                                .publish_for_recipient(&nickname, &message_request)
                                                .await {
                                                error!("failed to publish message: {:?}", e);
                                            }
                                        }
                                    }
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
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    context: WsRequestContext,
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
                    Some(Ok(item)) => {
                        let message = Binary(item.clone());
                        let mut sender = sender.write().await;
                        let _ = sender.send(message).await;
                        if let Err(e) = event_service.publish_for_storage(item.as_slice()).await {
                            error!("Failed to publish message for storage: {:?}", e);
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
        Ok(_) => debug!("Consumer '{:?}' closed", consumer_tag),
        Err(e) => error!("Failed to close consumer: {:?}", e),
    };
}
