use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws;
use axum::extract::ws::Message::{Binary, Close, Text};
use axum::extract::ws::WebSocket;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, Stream, StreamExt};
use log::{debug, error, warn};
use serde_json::from_str;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio::try_join;

use self::service::EventService;
use crate::event::model::{Command, Event, Queue};
use crate::integration::model::{CacheKey, Keyspace};
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{auth, chat, integration, message, user};

pub mod api;
pub mod service;

mod context;
mod model;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("missing user info")]
    MissingUserInfo,
    #[error("not a message owner")]
    NotOwner,
    #[error("not a message recipient")]
    NotRecipient,
    #[error("missing amqp channel")]
    MissingAmqpChannel,

    _Auth(#[from] auth::Error),
    _Chat(#[from] chat::Error),
    _Integration(#[from] integration::Error),
    _Message(#[from] message::Error),
    _User(#[from] user::Error),

    _ParseJson(#[from] serde_json::Error),
    _Lapin(#[from] lapin::Error),
    _Redis(#[from] redis::RedisError),
}

async fn handle_socket(ws: WebSocket, event_service: EventService, user_service: UserService) {
    let (sender, receiver) = ws.split();
    let ctx = context::Ws::new();

    let read_task = tokio::spawn(read(ctx.clone(), receiver, event_service.clone()));
    let write_task = tokio::spawn(write(
        ctx.clone(),
        sender,
        event_service.clone(),
        user_service.clone(),
    ));

    match try_join!(read_task, write_task) {
        Ok(_) => debug!("ws disconnected gracefully"),
        Err(e) => error!("ws disconnected with error: {}", e),
    }

    remove_online_user(&ctx, user_service).await
}

async fn read(ctx: context::Ws, mut receiver: SplitStream<WebSocket>, event_service: EventService) {
    loop {
        tokio::select! {
            // close is notified => stop 'read' task
            _ = ctx.close.notified() => break,

            // read next frame from ws connection
            frame = receiver.next() => {
                if let Some(message) = frame {
                    match message {
                        Err(e) => {
                            error!("failed to read ws frame: {:?}", e);
                            ctx.close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Close(_)) => {
                            debug!("ws connection closed by client");
                            ctx.close.notify_one(); // notify 'write' task to stop
                            break;
                        },
                        Ok(Text(content)) => {
                            if let Err(e) = handle_text_frame(&ctx, content, event_service.clone()).await {
                                error!("failed to handle text frame: {:?}", e);
                                ctx.close.notify_one(); // notify 'write' task to stop
                                break;
                            }
                        },
                        Ok(Binary(content)) => {
                            warn!("received binary ws frame: {:?}", content);
                        }
                        Ok(wtf) => warn!("received non-text ws frame: {:?}", wtf)
                    }
                }
            }
        }
    }
}

async fn write(
    ctx: context::Ws,
    sender: SplitSink<WebSocket, ws::Message>,
    event_service: EventService,
    user_service: UserService,
) {
    // wait for login notification or close
    tokio::select! {
        // close is notified => stop 'write' task
        _ = ctx.close.notified() => return,

        // TODO: uncomment
        // // didn't receive login notification within 5 seconds => stop 'write' task
        // _ = sleep(Duration::from_secs(5)) => {
        //     ctx.close.notify_one(); // notify 'read' task to stop
        //     return;
        // },

        // logged in => break the wait loop and start writing
        _ = ctx.login.notified() => {
            add_online_user(&ctx, user_service.clone()).await;
        },
    }

    let user_info = ctx
        .get_user_info()
        .await
        .expect("user info has to be set when logged in");

    let messages_queue = Queue::Messages(user_info.sub.to_owned());

    let mut event_stream = match event_service.read(&ctx, &messages_queue).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to create consumer of events: {:?}", e);
            ctx.close.notify_one();
            return;
        }
    };

    let mut online_status_changes = match listen_online_status_change().await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to listen online status changes: {:?}", e);
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
                    Some(Err(e)) => error!("Failed to read online status change: {:?}", e),
                    Some(Ok(msg)) => {
                        debug!("{:?}" ,msg);
                        publish_online_users(&ctx, &user_info, user_service.clone(), event_service.clone()).await;
                    }
                }
            },
            // new event is received from queue => send it to the client
            item = event_stream.next() => {
                match item {
                    None => break,
                    Some(Err(e)) => error!("Failed to read event from queue: {:?}", e),
                    Some(Ok(event)) => send_event(sender.clone(), &event).await,
                }
            },
        }
    }

    match event_service.close_channel(&ctx).await {
        Ok(_) => debug!("Channel closed"),
        Err(e) => error!("Failed to close channel: {:?}", e),
    }
}

async fn handle_text_frame(
    ctx: &context::Ws,
    content: String,
    event_service: EventService,
) -> Result<()> {
    if let Ok(command) = from_str::<Command>(content.as_str()) {
        return event_service.handle_command(ctx, command).await;
    }
    warn!("skipping frame, content is malformed: {content}");
    Ok(())
}

async fn send_event(sender: Arc<RwLock<SplitSink<WebSocket, ws::Message>>>, event: &Event) {
    debug!("sending event: {:?}", event);
    match serde_json::to_string(&event) {
        Ok(event) => {
            let mut sender = sender.write().await;
            if let Err(e) = sender.send(Text(event)).await {
                error!("Failed to send event to client: {:?}", e);
            }
        }
        Err(e) => error!("Failed to serialize event: {:?}", e),
    }
}

async fn publish_online_users(
    ctx: &context::Ws,
    user_info: &UserInfo,
    user_service: UserService,
    event_service: EventService,
) {
    if let Ok(users) = user_service.get_online_friends(&user_info.sub).await {
        if ctx.same_online_friends(&users).await {
            return;
        }

        match event_service
            .publish_event(
                ctx,
                &Queue::Messages(user_info.sub.to_owned()),
                &Event::OnlineUsers {
                    users: users.clone(),
                },
            )
            .await
        {
            Ok(_) => ctx.set_online_friends(users).await,
            Err(e) => error!("failed to publish online users event: {:?}", e),
        }
    }
}

async fn add_online_user(ctx: &context::Ws, user_service: UserService) {
    if let Some(user_info) = ctx.get_user_info().await {
        debug!("adding to online users: {}", &user_info.sub);
        if let Err(e) = user_service.add_online_user(&user_info.sub).await {
            error!("Failed to add user to online users: {:?}", e);
        }
    }
}

async fn remove_online_user(ctx: &context::Ws, user_service: UserService) {
    if let Some(user_info) = ctx.get_user_info().await {
        debug!("removing from online users: {}", &user_info.sub);
        if let Err(e) = user_service.remove_online_user(&user_info.sub).await {
            error!("Failed to remove user from online users: {:?}", e);
        }
    }
}

type OnlineStatusChangedStream = Pin<Box<dyn Stream<Item = Result<redis::Msg>> + Send>>;

// FIXME
async fn listen_online_status_change() -> Result<OnlineStatusChangedStream> {
    let config = crate::integration::cache::Config::env().unwrap_or_default();
    let client = crate::integration::cache::init_client(&config).await?;
    let mut con = client.get_async_connection().await?;

    enable_keyspace_events(&mut con).await?;

    let mut pubsub = con.into_pubsub();
    pubsub
        .psubscribe(Keyspace::new(CacheKey::UsersOnline))
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

async fn enable_keyspace_events(con: &mut redis::aio::Connection) -> Result<()> {
    redis::cmd("CONFIG")
        .arg("SET")
        .arg("notify-keyspace-events")
        .arg("KEAg")
        .query_async(con)
        .await
        .map(|_: ()| ())
        .map_err(Error::from)
}
