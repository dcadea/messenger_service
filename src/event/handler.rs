use axum::http::StatusCode;

use super::Error;

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::NotOwner | Error::NotRecipient => Self::FORBIDDEN,
            Error::_Axum(_) | Error::_NatsSub(_) | Error::_SerdeJson(_) => {
                Self::INTERNAL_SERVER_ERROR
            }
        }
    }
}

pub mod sse {
    use crate::event::{self, Notification, Subject};
    use crate::{auth, user};
    use async_stream;
    use axum::Extension;
    use axum::extract::State;
    use axum::response::sse;
    use futures::{Stream, StreamExt};
    use log::{debug, error};
    use maud::Render;
    use tokio::time;

    use std::time::Duration;

    const ONLINE_NOTI_INTERVAL: Duration = Duration::from_secs(15);

    pub async fn notifications(
        auth_user: Extension<auth::User>,
        user_service: State<user::Service>,
        event_service: State<event::Service>,
    ) -> sse::Sse<impl Stream<Item = crate::Result<sse::Event>>> {
        let auth_id = auth_user.id().clone();

        let stream = async_stream::try_stream! {
            let mut noti_stream = event_service
                .subscribe_noti(&Subject::Notifications(&auth_id))
                .await?;

            let _osd = OnlineStatusDropper(&auth_id, &user_service);
            let mut interval = time::interval(ONLINE_NOTI_INTERVAL);
            loop {
                tokio::select! {
                    next = noti_stream.next() => {
                        if let Some(noti) = next {
                            match noti {
                                Ok(n) => yield sse::Event::from(n),
                                Err(e) => error!("Error reading notification from stream: {e:?}"),
                            }
                        }
                    },
                    _ = interval.tick() => user_service.notify_online(&auth_id).await
                }
            }
            // _osd drops here
        };

        debug!("SSE connected for {:?}", auth_user.sub());
        sse::Sse::new(stream).keep_alive(
            sse::KeepAlive::new()
                .interval(Duration::from_secs(2))
                .text("sse-ping"),
        )
    }

    struct OnlineStatusDropper<'a>(&'a user::Id, &'a user::Service);

    impl Drop for OnlineStatusDropper<'_> {
        fn drop(&mut self) {
            let sub = self.0.clone();
            let user_service = self.1.clone();

            debug!("SSE dropped for {sub:?}");

            tokio::spawn(async move {
                user_service.notify_offline(&sub).await;
            });
        }
    }

    impl From<Notification> for sse::Event {
        fn from(noti: Notification) -> Self {
            let evt = match &noti {
                Notification::OnlineStatusChange(f) => &format!("onlineStatusChange:{}", f.id()),
                Notification::NewTalk(_) => "newTalk",
                Notification::NewMessage { talk_id, .. } => &format!("newMessage:{}", &talk_id),
            };

            Self::default().event(evt).data(noti.render().into_string())
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
    use axum::extract::ws;
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
    use log::{debug, error};
    use maud::Render;
    use tokio::sync::Notify;

    pub async fn talk(
        auth_user: Extension<auth::User>,
        ws: WebSocketUpgrade,
        Path(talk_id): Path<talk::Id>,
        State(user_service): State<user::Service>,
        State(event_service): State<event::Service>,
        State(message_service): State<message::Service>,
        State(talk_service): State<talk::Service>,
    ) -> crate::Result<Response> {
        debug!("Upgrading to WS for talk: {}", &talk_id);
        user_service.check_member(&talk_id, &auth_user).await?;

        let auth_id = auth_user.id().clone();
        Ok(ws.on_upgrade(move |socket| async {
            let (sender, recv) = socket.split();
            let close = Arc::new(Notify::new());

            tokio::spawn(send(
                auth_id,
                talk_id.clone(),
                sender,
                event_service,
                message_service,
                talk_service,
                close.clone(),
            ));

            tokio::spawn(receive(talk_id, recv, close));
        }))
    }

    async fn send(
        auth_id: user::Id,
        talk_id: talk::Id,
        mut sender: SplitSink<WebSocket, ws::Message>,
        event_service: event::Service,
        message_service: message::Service,
        talk_service: talk::Service,
        close: Arc<Notify>,
    ) -> event::Result<()> {
        let mut msg_stream = event_service
            .subscribe_event(&Subject::Messages(&auth_id, &talk_id))
            .await?;

        loop {
            tokio::select! {
                () = close.notified() => break,
                next = msg_stream.next() => {
                    let Some(msg) = next else { break };

                    let msg = match msg {
                        Ok(m) => m,
                        Err(e) => {
                            error!("Error reading message event from stream: {e:?}");
                            continue;
                        },
                    };

                    let markup = msg.render().into_string();
                    if let Err(e) = sender.send(ws::Message::Text(markup.into())).await {
                        error!("Failed to send event message to client: {e}");
                        break;
                    }

                    if let Message::New(msg) = msg {
                        let talk_id = msg.talk_id().clone();
                        match message_service.mark_as_seen(&auth_id, &[msg]).await {
                            Ok(seen_qty) => {
                                if seen_qty == 0 {
                                    continue;
                                }

                                if let Err(e) = talk_service.mark_as_seen(&talk_id) {
                                    error!("Failed to mark talk as seen: {e}");
                                }
                            }
                            Err(e) => error!("Failed to mark message as seen: {e}"),
                        }
                    }
                }
            }
        }

        sender.flush().await?;
        debug!("WS send task stopped for talk {talk_id:?}");

        Ok(())
    }

    async fn receive(talk_id: talk::Id, mut recv: SplitStream<WebSocket>, close: Arc<Notify>) {
        while let Some(msg) = recv.next().await {
            match msg {
                Err(e) => {
                    error!("Failed to read WS frame: {e}");
                    break;
                }
                Ok(ws::Message::Close(c)) => {
                    if let Some(cf) = c {
                        debug!("Client sent {cf:?}");
                    } else {
                        debug!("Client sent close message without CloseFrame");
                    }
                    close.notify_waiters();
                    break;
                }
                Ok(frame) => debug!("Received WS frame: {frame:?}"),
            }
        }

        debug!("WS receive task stopped for talk {talk_id:?}");
    }
}
