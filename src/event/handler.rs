use axum::extract::ws::{self, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::Extension;
use log::{debug, error, warn};
use serde_json::from_str;
use tokio::sync::{Notify, RwLock};

use super::model::{Command, Notification, Queue};
use super::service::EventService;
use crate::event::markup;
use crate::user;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use axum::extract::ws::Message::{Binary, Close, Text};
use tokio::try_join;

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};

use std::sync::Arc;

pub async fn ws(
    Extension(user_info): Extension<UserInfo>,
    ws: WebSocketUpgrade,
    State(event_service): State<EventService>,
    State(user_service): State<UserService>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(user_info.sub, socket, event_service, user_service))
}

async fn handle_socket(
    logged_sub: user::Sub,
    ws: WebSocket,
    event_service: EventService,
    user_service: UserService,
) {
    if let Err(e) = user_service.add_online_user(&logged_sub).await {
        error!("Failed to add user to online users: {e}");
    }

    let (sender, receiver) = ws.split();

    let close = Arc::new(Notify::new());
    let read_task = tokio::spawn(read(close.clone(), receiver, event_service.clone()));
    let write_task = tokio::spawn(write(
        close.clone(),
        logged_sub.to_owned(),
        sender,
        event_service.clone(),
        user_service.clone(),
    ));

    match try_join!(tokio::spawn(read_task), tokio::spawn(write_task)) {
        Ok(_) => debug!("WS disconnected gracefully"),
        Err(e) => error!("WS disconnected with error: {e}"),
    }

    if let Err(e) = user_service.remove_online_user(&logged_sub).await {
        error!("Failed to remove user from online users: {e}");
    }
}

async fn read(
    close: Arc<Notify>,
    mut receiver: SplitStream<WebSocket>,
    event_service: EventService,
) {
    loop {
        tokio::select! {
            // close is notified => stop 'read' task
            _ = close.notified() => break,

            // read next frame from WS connection
            frame = receiver.next() => {
                if let Some(message) = frame {
                    match message {
                        Err(e) => {
                            error!("Failed to read WS frame: {e}");
                            close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Close(frame)) => {
                            debug!("WS connection closed by client: {:?}", frame);
                            close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Text(content)) => {
                            if let Err(e) = handle_text_frame(content, event_service.clone()).await {
                                error!("Failed to handle text frame: {e}");
                                close.notify_one(); // notify 'write' task to stop
                                break;
                            }
                        },
                        Ok(Binary(content)) => {
                            warn!("Received binary WS frame: {:?}", content);
                        }
                        Ok(wtf) => warn!("Received non-text WS frame: {:?}", wtf)
                    }
                }
            }
        }
    }
}

async fn handle_text_frame(content: String, event_service: EventService) -> super::Result<()> {
    if let Ok(command) = from_str::<Command>(content.as_str()) {
        return event_service.handle_command(command).await;
    }
    warn!("Skipping text frame, content is malformed: {content}");
    Ok(())
}

async fn write(
    close: Arc<Notify>,
    logged_sub: user::Sub,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    user_service: UserService,
) {
    let messages_queue = Queue::Messages(logged_sub.clone());

    let mut noti_stream = match event_service.read(&messages_queue).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to create AMQP consumer of notifications: {e}");
            close.notify_one();
            return;
        }
    };

    let mut online_status_changes = match event_service.listen_online_status_change().await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to listen online status changes: {e}");
            close.notify_one();
            return;
        }
    };

    let sender = Arc::new(RwLock::new(sender));
    loop {
        tokio::select! {
            // close is notified => stop 'write' task
            _ = close.notified() => break,

            // push new list of online users when somebody logs in or out
            status = online_status_changes.next() => {
                match status {
                    None => continue,
                    Some(Err(e)) => error!("Failed to read online status change: {e}"),
                    Some(Ok(_)) => {
                        publish_online_friends(&logged_sub, user_service.clone(), event_service.clone()).await;
                    }
                }
            },
            // new notification is received from queue => send it to the client
            item = noti_stream.next() => {
                match item {
                    None => break,
                    Some(None) => warn!("Looks like there was an issue reading from subject..."),
                    Some(Some(noti)) => {
                        debug!("Sending notification: {:?}", noti);

                        let mut sender = sender.write().await;
                        let noti_markup = markup::noti_item(&noti, &logged_sub);

                        if let Err(e) = sender.send(Text(noti_markup.into_string())).await {
                            error!("Failed to send notification to client: {e}");
                        }
                    }
                }
            },
        }
    }
}

async fn publish_online_friends(
    logged_sub: &user::Sub,
    user_service: UserService,
    event_service: EventService,
) {
    if let Ok(friends) = user_service.get_online_friends(logged_sub).await {
        if friends.is_empty() {
            return;
        }

        if let Err(e) = event_service
            .publish_noti(
                &Queue::Messages(logged_sub.clone()),
                &Notification::OnlineFriends {
                    friends: friends.clone(),
                },
            )
            .await
        {
            error!("Failed to publish online users notification: {e}");
        }
    }
}
