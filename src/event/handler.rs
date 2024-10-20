use axum::extract::ws::{self, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::Extension;
use log::{debug, error, warn};
use serde_json::from_str;
use tokio::sync::RwLock;

use super::context;
use super::model::{Command, Notification, Queue};
use super::service::EventService;
use crate::event::markup;
use crate::integration::cache;
use crate::user;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use axum::extract::ws::Message::{Binary, Close, Text};
use tokio::try_join;

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, Stream, StreamExt};

use std::pin::Pin;
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
    let ctx = match event_service.create_channel().await {
        Ok(channel) => {
            let ctx = context::Ws::new(logged_sub.to_owned(), channel);
            if let Err(e) = user_service.add_online_user(&logged_sub).await {
                error!("Failed to add user to online users: {e}");
            }
            ctx
        }
        Err(e) => {
            error!("Failed to create AMQP channel. Aborting WS connection: {e}");
            ws.close().await.expect("Failed to close WS connection");
            return;
        }
    };

    let (sender, receiver) = ws.split();

    let read_task = tokio::spawn(read(ctx.clone(), receiver, event_service.clone()));
    let write_task = tokio::spawn(write(
        ctx.clone(),
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
    match event_service.close_channel(&ctx).await {
        Ok(_) => debug!("AMQP channel closed gracefully"),
        Err(e) => error!("Failed to close AMQP channel: {e}"),
    }
}

async fn read(ctx: context::Ws, mut receiver: SplitStream<WebSocket>, event_service: EventService) {
    loop {
        tokio::select! {
            // close is notified => stop 'read' task
            _ = ctx.close.notified() => break,

            // read next frame from WS connection
            frame = receiver.next() => {
                if let Some(message) = frame {
                    match message {
                        Err(e) => {
                            error!("Failed to read WS frame: {e}");
                            ctx.close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Close(frame)) => {
                            debug!("WS connection closed by client: {:?}", frame);
                            ctx.close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Text(content)) => {
                            if let Err(e) = handle_text_frame(&ctx, content, event_service.clone()).await {
                                error!("Failed to handle text frame: {e}");
                                ctx.close.notify_one(); // notify 'write' task to stop
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

async fn handle_text_frame(
    ctx: &context::Ws,
    content: String,
    event_service: EventService,
) -> super::Result<()> {
    if let Ok(command) = from_str::<Command>(content.as_str()) {
        return event_service.handle_command(ctx, command).await;
    }
    warn!("Skipping text frame, content is malformed: {content}");
    Ok(())
}

async fn write(
    ctx: context::Ws,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    user_service: UserService,
) {
    let messages_queue = Queue::Messages(ctx.logged_sub.clone());

    let mut noti_stream = match event_service.read(&ctx, &messages_queue).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to create AMQP consumer of notifications: {e}");
            ctx.close.notify_one();
            return;
        }
    };

    let mut online_status_changes = match listen_online_status_change().await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to listen online status changes: {e}");
            ctx.close.notify_one();
            return;
        }
    };

    let sender = Arc::new(RwLock::new(sender));
    loop {
        tokio::select! {
            // close is notified => stop 'write' task
            _ = ctx.close.notified() => break,

            // push new list of online users when somebody logs in or out
            status = online_status_changes.next() => {
                match status {
                    None => continue,
                    Some(Err(e)) => error!("Failed to read online status change: {e}"),
                    Some(Ok(_)) => {
                        publish_online_friends(&ctx, user_service.clone(), event_service.clone()).await;
                    }
                }
            },
            // new notification is received from queue => send it to the client
            item = noti_stream.next() => {
                match item {
                    None => break,
                    Some(Err(e)) => error!("Failed to read notification from queue: {e}"),
                    Some(Ok(noti)) => {
                        debug!("Sending notification: {:?}", noti);

                        let mut sender = sender.write().await;
                        let noti_markup = markup::noti_item(&noti, &ctx.logged_sub);

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
    ctx: &context::Ws,
    user_service: UserService,
    event_service: EventService,
) {
    let logged_sub = ctx.logged_sub.to_owned();

    if let Ok(friends) = user_service.get_online_friends(&logged_sub).await {
        if friends.is_empty() {
            return;
        }

        if let Err(e) = event_service
            .publish_noti(
                ctx,
                &Queue::Messages(logged_sub),
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

type OnlineStatusChangedStream = Pin<Box<dyn Stream<Item = super::Result<redis::Msg>> + Send>>;

// FIXME: implement online functionality properly
async fn listen_online_status_change() -> super::Result<OnlineStatusChangedStream> {
    let config = crate::integration::cache::Config::env().unwrap_or_default();
    let client = crate::integration::cache::init_client(&config).await?;
    let mut con = client.get_multiplexed_async_connection().await?;

    enable_keyspace_events(&mut con).await?;

    let mut pubsub = client.get_async_pubsub().await?;

    pubsub
        .psubscribe(cache::Keyspace::new(cache::Key::UsersOnline))
        .await?;

    let stream = pubsub
        .into_on_message()
        .map(|msg| {
            debug!("Received keyspace message: {:?}", msg);
            Ok(msg)
        })
        .boxed();

    Ok(Box::pin(stream))
}

async fn enable_keyspace_events(con: &mut redis::aio::MultiplexedConnection) -> super::Result<()> {
    redis::cmd("CONFIG")
        .arg("SET")
        .arg("notify-keyspace-events")
        .arg("KEAg")
        .query_async(con)
        .await
        .map(|_: ()| ())
        .map_err(super::Error::from)
}
