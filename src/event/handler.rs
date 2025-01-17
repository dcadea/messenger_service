use super::service::EventService;
use super::{Message, Notification, Queue};

use crate::chat::service::ChatValidator;
use crate::message::service::MessageService;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{chat, user};
use axum::extract::ws::Message::{Close, Text};
use axum::extract::ws::{self, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;
use axum::Extension;
use log::{debug, error, warn};
use maud::Render;
use tokio::sync::{Notify, RwLock};
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
    ws.on_upgrade(move |socket| handle_global(user_info.sub, socket, event_service, user_service))
}

async fn handle_global(
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
    let read = read(close.clone(), receiver);
    let write = write_global(
        close.clone(),
        logged_sub.to_owned(),
        sender,
        event_service.clone(),
        user_service.clone(),
    );

    match try_join!(tokio::spawn(read), tokio::spawn(write)) {
        Ok(_) => debug!("WS disconnected gracefully"),
        Err(e) => error!("WS disconnected with error: {e}"),
    }

    if let Err(e) = user_service.remove_online_user(&logged_sub).await {
        error!("Failed to remove user from online users: {e}");
    }
}

async fn write_global(
    close: Arc<Notify>,
    logged_sub: user::Sub,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    user_service: UserService,
) {
    let mut noti_stream = match event_service
        .subscribe::<Notification>(Queue::Notifications(logged_sub.clone()))
        .await
    {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to subscribe to queue: {e}");
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
            noti = noti_stream.next() => {
                match noti {
                    None => break,
                    Some(None) => warn!("Looks like there was an issue reading from subject..."),
                    Some(Some(noti)) => {
                        let mut sender = sender.write().await;

                        let markup = noti.render();
                        if let Err(e) = sender.send(Text(markup.into_string())).await {
                            error!("Failed to send notification to client: {e}");
                        }
                    }
                }
            },
        }
    }
}

pub async fn ws_chat(
    Extension(user_info): Extension<UserInfo>,
    ws: WebSocketUpgrade,
    Path(chat_id): Path<chat::Id>,
    State(chat_validator): State<ChatValidator>,
    State(event_service): State<EventService>,
    State(message_service): State<MessageService>,
) -> crate::Result<Response> {
    chat_validator
        .check_member(&chat_id, &user_info.sub)
        .await?;

    Ok(ws.on_upgrade(move |socket| {
        handle_chat(
            user_info.sub,
            chat_id,
            socket,
            event_service,
            message_service,
        )
    }))
}

async fn handle_chat(
    logged_sub: user::Sub,
    chat_id: chat::Id,
    ws: WebSocket,
    event_service: EventService,
    message_service: MessageService,
) {
    let (sender, receiver) = ws.split();

    let close = Arc::new(Notify::new());
    let read = read(close.clone(), receiver);
    let write = write_chat(
        close.clone(),
        logged_sub,
        chat_id,
        sender,
        event_service,
        message_service,
    );

    match try_join!(tokio::spawn(read), tokio::spawn(write)) {
        Ok(_) => debug!("WS chat disconnected gracefully"),
        Err(e) => error!("WS chat disconnected with error: {e}"),
    }
}

async fn write_chat(
    close: Arc<Notify>,
    logged_sub: user::Sub,
    chat_id: chat::Id,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    message_service: MessageService,
) {
    let mut messages_stream = match event_service
        .subscribe::<Message>(Queue::Messages(logged_sub.clone(), chat_id))
        .await
    {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to subscribe to queue: {e}");
            close.notify_one();
            return;
        }
    };

    let sender = Arc::new(RwLock::new(sender));
    loop {
        tokio::select! {
            // close is notified => stop 'write' task
            _ = close.notified() => break,

            // new message is received from queue => send it to the client
            msg = messages_stream.next() => {
                match msg {
                    None => break,
                    Some(None) => warn!("Looks like there was an issue reading from subject..."),
                    Some(Some(msg)) => {
                        let mut sender = sender.write().await;

                        let markup = msg.render();
                        if let Err(e) = sender.send(Text(markup.into_string())).await {
                            error!("Failed to send notification to client: {e}");
                        }

                        if let Message::New { msg } = msg {
                            if let Err(e) = message_service.mark_as_seen(&logged_sub, &[msg]).await {
                                error!("Failed to mark message as seen: {e}");
                            }
                        }
                    }
                }
            },
        }
    }
}

async fn read(close: Arc<Notify>, mut receiver: SplitStream<WebSocket>) {
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
                        Ok(frame) => warn!("Received WS frame: {:?}", frame)
                    }
                }
            }
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
            .publish(
                Queue::Notifications(logged_sub.clone()),
                Notification::OnlineFriends {
                    friends: friends.clone(),
                },
            )
            .await
        {
            error!("Failed to publish online users notification: {e}");
        }
    }
}
