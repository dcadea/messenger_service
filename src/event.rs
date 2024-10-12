use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws;
use axum::extract::ws::Message::{Binary, Close, Text};
use axum::extract::ws::WebSocket;
use axum::routing::get;
use axum::Router;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, Stream, StreamExt};
use log::{debug, error, warn};
use model::Command;
use serde_json::from_str;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio::try_join;

use self::service::EventService;
use crate::event::model::{Event, Queue};
use crate::integration::cache;
use crate::state::AppState;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{auth, chat, integration, message, user};

type Result<T> = std::result::Result<T, Error>;

pub(crate) fn endpoints<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/ws", get(handler::ws))
        .with_state(state)
}

pub(self) mod handler {
    use axum::extract::{State, WebSocketUpgrade};
    use axum::response::Response;

    use crate::user::service::UserService;

    use super::handle_socket;
    use super::service::EventService;

    pub async fn ws(
        ws: WebSocketUpgrade,
        State(event_service): State<EventService>,
        State(user_service): State<UserService>,
    ) -> crate::Result<Response> {
        Ok(ws.on_upgrade(move |socket| handle_socket(socket, event_service, user_service)))
    }
}

pub(crate) mod service {
    use std::io;
    use std::sync::Arc;

    use futures::TryStreamExt;
    use lapin::options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions,
    };
    use lapin::types::FieldTable;
    use lapin::{BasicProperties, Channel, Connection};
    use log::debug;
    use tokio::sync::RwLock;

    use crate::auth::service::AuthService;
    use crate::chat::service::ChatService;
    use crate::message::model::{Message, MessageDto};
    use crate::message::service::MessageService;
    use crate::user::service::UserService;
    use crate::{event, user};

    use super::context;
    use super::model::{Command, Event, EventStream, Queue};

    #[derive(Clone)]
    pub struct EventService {
        amqp_con: Arc<RwLock<Connection>>,
        auth_service: Arc<AuthService>,
        chat_service: Arc<ChatService>,
        message_service: Arc<MessageService>,
        user_service: Arc<UserService>,
    }

    impl EventService {
        pub fn new(
            amqp_con: RwLock<Connection>,
            auth_service: AuthService,
            chat_service: ChatService,
            message_service: MessageService,
            user_service: UserService,
        ) -> Self {
            Self {
                amqp_con: Arc::new(amqp_con),
                auth_service: Arc::new(auth_service),
                chat_service: Arc::new(chat_service),
                message_service: Arc::new(message_service),
                user_service: Arc::new(user_service),
            }
        }
    }

    impl EventService {
        pub(super) async fn handle_command(
            &self,
            ctx: &context::Ws,
            command: Command,
        ) -> super::Result<()> {
            debug!("handling command: {:?}", command);
            match ctx.get_user_info().await {
                None => {
                    // if let Command::Auth { token } = command {
                    // TODO: revert
                    // let claims = self.auth_service.validate(&token).await?;
                    // let user_info = self.user_service.find_user_info(&claims.sub).await?;
                    let user_info = self
                        .user_service
                        .find_user_info(&user::Sub("github|10639696".to_string()))
                        .await?;
                    ctx.set_user_info(user_info).await;
                    ctx.set_channel(self.create_channel().await?).await;
                    ctx.login.notify_one();
                    return Ok(());
                    // }

                    // Err(event::Error::MissingUserInfo)
                }
                Some(user_info) => match command {
                    Command::Auth { .. } => {
                        debug!("received auth request with user info already set, ignoring");
                        Ok(())
                    }
                    Command::CreateMessage {
                        chat_id,
                        recipient,
                        text,
                    } => {
                        let owner = user_info.sub;

                        self.chat_service
                            .check_members(&chat_id, [owner.clone(), recipient.clone()])
                            .await?;

                        let message = self
                            .message_service
                            .create(&Message::new(
                                chat_id,
                                owner.clone(),
                                recipient.clone(),
                                &text,
                            ))
                            .await?;

                        let owner_messages = Queue::Messages(owner);
                        let recipient_messages = Queue::Messages(recipient);
                        let event = Event::NewMessage {
                            message: MessageDto::from(message.clone()),
                        };

                        use futures::TryFutureExt;

                        tokio::try_join!(
                            self.publish_event(ctx, &owner_messages, &event),
                            self.publish_event(ctx, &recipient_messages, &event),
                            self.chat_service
                                .update_last_message(&message)
                                .map_err(event::Error::from)
                        )
                        .map(|_| ())
                    }
                    Command::UpdateMessage { id, text } => {
                        let message = self.message_service.find_by_id(&id).await?;
                        if message.owner != user_info.sub {
                            return Err(event::Error::NotOwner);
                        }

                        self.message_service.update(&id, &text).await?;

                        let owner_messages = Queue::Messages(message.owner);
                        let recipient_messages = Queue::Messages(message.recipient);
                        let event = Event::UpdatedMessage { id, text };

                        tokio::try_join!(
                            self.publish_event(ctx, &owner_messages, &event),
                            self.publish_event(ctx, &recipient_messages, &event)
                        )
                        .map(|_| ())
                    }
                    Command::DeleteMessage { id } => {
                        let message = self.message_service.find_by_id(&id).await?;
                        if message.owner != user_info.sub {
                            return Err(event::Error::NotOwner);
                        }
                        self.message_service.delete(&id).await?;

                        let owner_messages = Queue::Messages(message.owner);
                        let recipient_messages = Queue::Messages(message.recipient);
                        let event = Event::DeletedMessage { id };

                        tokio::try_join!(
                            self.publish_event(ctx, &owner_messages, &event),
                            self.publish_event(ctx, &recipient_messages, &event)
                        )
                        .map(|_| ())
                    }
                    Command::MarkAsSeenMessage { id } => {
                        let message = self.message_service.find_by_id(&id).await?;
                        if message.recipient != user_info.sub {
                            return Err(event::Error::NotRecipient);
                        }
                        self.message_service.mark_as_seen(&id).await?;

                        let owner_messages = Queue::Messages(message.owner);
                        self.publish_event(ctx, &owner_messages, &Event::SeenMessage { id })
                            .await
                    }
                },
            }
        }
    }

    impl EventService {
        pub async fn read(&self, ctx: &context::Ws, q: &Queue) -> super::Result<EventStream> {
            self.ensure_queue_exists(ctx, q).await?;

            let consumer = ctx
                .get_channel()
                .await?
                .basic_consume(
                    &q.to_string(),
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await?;

            let stream = consumer
                .and_then(|delivery| async move {
                    let event = serde_json::from_slice::<Event>(&delivery.data)
                        .map_err(|e| lapin::Error::IOError(Arc::new(io::Error::from(e))))?;
                    delivery.ack(BasicAckOptions::default()).await?;
                    Ok(event)
                })
                .map_err(event::Error::from);

            Ok(Box::pin(stream))
        }

        pub async fn close_channel(&self, ctx: &context::Ws) -> super::Result<()> {
            let channel = ctx.get_channel().await?;
            channel.close(200, "OK").await.map_err(event::Error::from)
        }

        pub async fn publish_event(
            &self,
            ctx: &context::Ws,
            q: &Queue,
            event: &Event,
        ) -> super::Result<()> {
            let payload = serde_json::to_vec(event)?;
            self.publish(ctx, q, payload.as_slice()).await
        }
    }

    impl EventService {
        async fn create_channel(&self) -> super::Result<Channel> {
            let conn = self.amqp_con.read().await;
            conn.create_channel().await.map_err(event::Error::from)
        }

        async fn publish(&self, ctx: &context::Ws, q: &Queue, payload: &[u8]) -> super::Result<()> {
            self.ensure_queue_exists(ctx, q).await?;
            ctx.get_channel()
                .await?
                .basic_publish(
                    "",
                    &q.to_string(),
                    BasicPublishOptions::default(),
                    payload,
                    BasicProperties::default(),
                )
                .await?;
            Ok(())
        }

        async fn ensure_queue_exists(&self, ctx: &context::Ws, q: &Queue) -> super::Result<()> {
            ctx.get_channel()
                .await?
                .queue_declare(
                    &q.to_string(),
                    QueueDeclareOptions {
                        auto_delete: true,
                        ..QueueDeclareOptions::default()
                    },
                    FieldTable::default(),
                )
                .await
                .map(|_| ())
                .map_err(event::Error::from)
        }
    }
}

mod context {
    use std::collections::HashSet;
    use std::sync::Arc;

    use tokio::sync::{Notify, RwLock};

    use crate::user::model::UserInfo;
    use crate::{event, user};

    #[derive(Clone)]
    pub struct Ws {
        user_info: Arc<RwLock<Option<UserInfo>>>,
        channel: Arc<RwLock<Option<lapin::Channel>>>,
        online_friends: Arc<RwLock<HashSet<user::Sub>>>,
        pub login: Arc<Notify>,
        pub close: Arc<Notify>,
    }

    impl Ws {
        pub fn new() -> Self {
            Self {
                user_info: Arc::new(RwLock::new(None)),
                channel: Arc::new(RwLock::new(None)),
                online_friends: Arc::new(RwLock::new(HashSet::new())),
                login: Arc::new(Notify::new()),
                close: Arc::new(Notify::new()),
            }
        }
    }

    impl Ws {
        pub async fn set_user_info(&self, user_info: UserInfo) {
            *self.user_info.write().await = Some(user_info);
        }

        pub async fn get_user_info(&self) -> Option<UserInfo> {
            self.user_info.read().await.clone()
        }

        pub async fn set_channel(&self, channel: lapin::Channel) {
            *self.channel.write().await = Some(channel);
        }

        pub async fn get_channel(&self) -> super::Result<lapin::Channel> {
            self.channel
                .read()
                .await
                .clone()
                .ok_or(event::Error::MissingAmqpChannel)
        }

        pub async fn set_online_friends(&self, friends: HashSet<user::Sub>) {
            *self.online_friends.write().await = friends;
        }

        pub async fn same_online_friends(&self, friends: &HashSet<user::Sub>) -> bool {
            let f = self.online_friends.read().await;
            f.symmetric_difference(friends).count() == 0
        }
    }
}

mod model {
    use std::collections::HashSet;
    use std::fmt::Display;
    use std::pin::Pin;

    use futures::Stream;
    use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
    use serde::{Deserialize, Serialize};

    use crate::message::model::MessageDto;
    use crate::{chat, message, user};

    pub type EventStream = Pin<Box<dyn Stream<Item = super::Result<Event>> + Send>>;

    #[derive(Clone)]
    pub enum Queue {
        Messages(user::Sub),
    }

    impl Display for Queue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Queue::Messages(sub) => write!(f, "messages:{sub}"),
            }
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum Command {
        Auth {
            token: String,
        },
        CreateMessage {
            chat_id: chat::Id,
            recipient: user::Sub,
            text: String,
        },
        UpdateMessage {
            id: message::Id,
            text: String,
        },
        DeleteMessage {
            id: message::Id,
        },
        MarkAsSeenMessage {
            id: message::Id,
        },
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum Event {
        NewMessage {
            message: MessageDto,
        },
        UpdatedMessage {
            #[serde(serialize_with = "serialize_object_id_as_hex_string")]
            id: message::Id,
            text: String,
        },
        DeletedMessage {
            #[serde(serialize_with = "serialize_object_id_as_hex_string")]
            id: message::Id,
        },
        SeenMessage {
            #[serde(serialize_with = "serialize_object_id_as_hex_string")]
            id: message::Id,
        },
        OnlineUsers {
            users: HashSet<user::Sub>,
        },
    }
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
) -> self::Result<()> {
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

type OnlineStatusChangedStream = Pin<Box<dyn Stream<Item = self::Result<redis::Msg>> + Send>>;

// FIXME
async fn listen_online_status_change() -> self::Result<OnlineStatusChangedStream> {
    let config = crate::integration::cache::Config::env().unwrap_or_default();
    let client = crate::integration::cache::init_client(&config).await?;
    let mut con = client.get_async_connection().await?;

    enable_keyspace_events(&mut con).await?;

    let mut pubsub = con.into_pubsub();
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

async fn enable_keyspace_events(con: &mut redis::aio::Connection) -> self::Result<()> {
    redis::cmd("CONFIG")
        .arg("SET")
        .arg("notify-keyspace-events")
        .arg("KEAg")
        .query_async(con)
        .await
        .map(|_: ()| ())
        .map_err(Error::from)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
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
