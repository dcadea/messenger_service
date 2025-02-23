pub mod sse {
    use crate::event::service::EventService;
    use crate::event::{Notification, Subject};
    use crate::user;
    use crate::user::model::UserInfo;
    use crate::user::service::UserService;
    use async_stream;
    use axum::Extension;
    use axum::extract::State;
    use axum::response::sse;
    use futures::{Stream, StreamExt};

    use std::convert::Infallible;
    use std::time::Duration;

    pub async fn notifications(
        Extension(user_info): Extension<UserInfo>,
        State(user_service): State<UserService>,
        event_service: State<EventService>,
    ) -> sse::Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
        let sub = user_info.sub;

        let stream = async_stream::stream! {
            let mut noti_stream = event_service
                .subscribe::<Notification>(&Subject::Notifications(&sub))
                .await
                .expect("failed to subscribe to subject"); // FIXME

            // FIXME: if user has two or more sessions
            // and closes one - user becomes offline (not ok)
            user_service.add_online_user(&sub).await;
            let _osd = OnlineStatusDropper(&sub, &user_service);

            loop {
                tokio::select! {
                    noti = noti_stream.next() => {
                        match noti {
                            Some(noti) => yield Ok(sse::Event::from(noti)),
                            None => continue,
                        }
                    }
                }
            }
            // _osd drops here
        };

        sse::Sse::new(stream).keep_alive(
            sse::KeepAlive::new()
                .interval(Duration::from_secs(2))
                .text("sse-ping"),
        )
    }

    struct OnlineStatusDropper<'a>(&'a user::Sub, &'a UserService);

    impl<'a> Drop for OnlineStatusDropper<'a> {
        fn drop(&mut self) {
            let sub = self.0.clone();
            let user_service = self.1.clone();

            tokio::spawn(async move {
                user_service.remove_online_user(&sub).await;
            });
        }
    }
}

pub mod ws {
    use std::sync::Arc;

    use crate::{
        chat::{
            self,
            service::{ChatService, ChatValidator},
        },
        event::{Message, Subject, service::EventService},
        message::service::MessageService,
        user::{self, model::UserInfo},
    };
    use axum::extract::ws::Message::{Close, Text};
    use axum::{
        Extension,
        extract::{Path, State, WebSocketUpgrade, ws::WebSocket},
        response::Response,
    };
    use futures::StreamExt;
    use futures::{
        SinkExt,
        stream::{SplitSink, SplitStream},
    };
    use log::{debug, error, warn};
    use maud::Render;
    use tokio::{
        sync::{Notify, RwLock},
        try_join,
    };

    pub async fn chat(
        Extension(user_info): Extension<UserInfo>,
        ws: WebSocketUpgrade,
        Path(chat_id): Path<chat::Id>,
        State(chat_validator): State<ChatValidator>,
        State(event_service): State<EventService>,
        State(message_service): State<MessageService>,
        State(chat_service): State<ChatService>,
    ) -> crate::Result<Response> {
        chat_validator
            .check_member(&chat_id, &user_info.sub)
            .await?;

        Ok(ws.on_upgrade(move |socket| {
            handle(
                user_info.sub,
                chat_id,
                socket,
                event_service,
                message_service,
                chat_service,
            )
        }))
    }

    async fn handle(
        logged_sub: user::Sub,
        chat_id: chat::Id,
        ws: WebSocket,
        event_service: EventService,
        message_service: MessageService,
        chat_service: ChatService,
    ) {
        let (sender, receiver) = ws.split();

        let close = Arc::new(Notify::new());
        let read = read(close.clone(), receiver);
        let write = write(
            close.clone(),
            logged_sub,
            chat_id,
            sender,
            event_service,
            message_service,
            chat_service,
        );

        match try_join!(tokio::spawn(read), tokio::spawn(write)) {
            Ok(_) => debug!("WS chat disconnected gracefully"),
            Err(e) => error!("WS chat disconnected with error: {e}"),
        }
    }

    async fn write(
        close: Arc<Notify>,
        logged_sub: user::Sub,
        chat_id: chat::Id,
        sender: SplitSink<WebSocket, axum::extract::ws::Message>,
        event_service: EventService,
        message_service: MessageService,
        chat_service: ChatService,
    ) {
        let mut messages_stream = match event_service
            .subscribe::<Message>(&Subject::Messages(&logged_sub, &chat_id))
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                error!("Failed to subscribe to subject: {e}");
                close.notify_one();
                return;
            }
        };

        let sender = Arc::new(RwLock::new(sender));
        loop {
            tokio::select! {
                // close is notified => stop 'write' task
                _ = close.notified() => break,

                // new message is received from subject => send it to the client
                msg = messages_stream.next() => {
                    match msg {
                        None => break,
                        Some(msg) => {
                            let mut sender = sender.write().await;

                            let markup = msg.render();
                            if let Err(e) = sender.send(Text(markup.into_string().into())).await {
                                error!("Failed to send notification to client: {e}");
                            }

                            if let Message::New(msg) = msg {
                                let chat_id = msg.chat_id.clone();
                                match message_service.mark_as_seen(&logged_sub, &[msg]).await {
                                    Ok(seen_qty) => {
                                        if seen_qty > 0 {
                                            if let Err(e) = chat_service.mark_as_seen(&chat_id).await {
                                                error!("Failed to mark chat as seen: {e}");
                                            }
                                        }
                                    }
                                    Err(e) => error!("Failed to mark message as seen: {e}"),
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
                            Ok(Close(_)) => {
                                debug!("WS connection closed by client");
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
}

// TODO: online users feature
// async fn publish_online_friends(
//     logged_sub: &user::Sub,
//     user_service: UserService,
//     event_service: EventService,
// ) {
//     if let Some(friends) = user_service.get_online_friends(logged_sub).await {
//         if friends.is_empty() {
//             return;
//         }

//         if let Err(e) = event_service
//             .publish(
//                 &Subject::Notifications(logged_sub.clone()),
//                 Notification::OnlineFriends {
//                     friends: friends.clone(),
//                 },
//             )
//             .await
//         {
//             error!("Failed to publish online users notification: {e}");
//         }
//     }
// }
