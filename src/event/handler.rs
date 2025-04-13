pub mod sse {
    use crate::event::{self, Subject};
    use crate::{auth, user};
    use async_stream;
    use axum::Extension;
    use axum::extract::State;
    use axum::response::sse;
    use futures::{Stream, StreamExt};
    use tokio::time;

    use std::convert::Infallible;
    use std::time::Duration;

    pub async fn notifications(
        Extension(auth_user): Extension<auth::User>,
        State(user_service): State<user::Service>,
        State(event_service): State<event::Service>,
    ) -> sse::Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
        let auth_sub = auth_user.sub;

        let stream = async_stream::stream! {
            let mut noti_stream = event_service
                .subscribe_noti(&Subject::Notifications(&auth_sub))
                .await
                .expect("failed to subscribe to subject"); // FIXME

            let _osd = OnlineStatusDropper(&auth_sub, &user_service);

            let mut interval = time::interval(Duration::from_secs(15));
            loop {
                tokio::select! {
                    noti = noti_stream.next() => {
                        match noti {
                            Some(noti) => yield Ok(sse::Event::from(noti)),
                            None => continue,
                        }
                    },
                    _ = interval.tick() => {
                        user_service.notify_online(&auth_sub).await;
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

    struct OnlineStatusDropper<'a>(&'a user::Sub, &'a user::Service);

    impl Drop for OnlineStatusDropper<'_> {
        fn drop(&mut self) {
            let sub = self.0.clone();
            let user_service = self.1.clone();

            tokio::spawn(async move {
                user_service.notify_offline(&sub).await;
            });
        }
    }
}

pub mod ws {
    use std::sync::Arc;

    use crate::{
        auth,
        event::{self, Message, Subject},
        message, talk, user,
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

    pub async fn talk(
        Extension(auth_user): Extension<auth::User>,
        ws: WebSocketUpgrade,
        Path(talk_id): Path<talk::Id>,
        State(talk_validator): State<talk::Validator>,
        State(event_service): State<event::Service>,
        State(message_service): State<message::Service>,
        State(talk_service): State<talk::Service>,
    ) -> crate::Result<Response> {
        talk_validator.check_member(&talk_id, &auth_user).await?;

        Ok(ws.on_upgrade(move |socket| {
            handle(
                auth_user.sub,
                talk_id,
                socket,
                event_service,
                message_service,
                talk_service,
            )
        }))
    }

    async fn handle(
        auth_sub: user::Sub,
        talk_id: talk::Id,
        ws: WebSocket,
        event_service: event::Service,
        message_service: message::Service,
        talk_service: talk::Service,
    ) {
        let (sender, receiver) = ws.split();

        let close = Arc::new(Notify::new());
        let read = read(close.clone(), receiver);
        let write = write(
            close.clone(),
            auth_sub,
            talk_id,
            sender,
            event_service,
            message_service,
            talk_service,
        );

        match try_join!(tokio::spawn(read), tokio::spawn(write)) {
            Ok(_) => debug!("WS talk disconnected gracefully"),
            Err(e) => error!("WS talk disconnected with error: {e}"),
        }
    }

    async fn write(
        close: Arc<Notify>,
        auth_sub: user::Sub,
        talk_id: talk::Id,
        sender: SplitSink<WebSocket, axum::extract::ws::Message>,
        event_service: event::Service,
        message_service: message::Service,
        talk_service: talk::Service,
    ) {
        let mut msg_stream = match event_service
            .subscribe_event(&Subject::Messages(&auth_sub, &talk_id))
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
                () = close.notified() => break,

                // new message is received from subject => send it to the client
                msg = msg_stream.next() => {
                    match msg {
                        None => break,
                        Some(msg) => {
                            let mut sender = sender.write().await;

                            let markup = msg.render();
                            if let Err(e) = sender.send(Text(markup.into_string().into())).await {
                                error!("Failed to send event message to client: {e}");
                            }

                            if let Message::New(msg) = msg {
                                let talk_id = msg.talk_id.clone();
                                match message_service.mark_as_seen(&auth_sub, &[msg]).await {
                                    Ok(seen_qty) => {
                                        if seen_qty > 0 {
                                            if let Err(e) = talk_service.mark_as_seen(&talk_id).await {
                                                error!("Failed to mark talk as seen: {e}");
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
                () = close.notified() => break,

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
                            Ok(frame) => warn!("Received WS frame: {frame:?}")
                        }
                    }
                }
            }
        }
    }
}
