use axum::http::StatusCode;

use super::Error;

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::NotOwner | Error::NotRecipient => StatusCode::FORBIDDEN,
            Error::_Axum(_) | Error::_NatsSub(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
    use log::debug;
    use maud::Render;
    use tokio::time;

    use std::time::Duration;

    const ONLINE_NOTI_INTERVAL: Duration = Duration::from_secs(15);

    pub async fn notifications(
        auth_user: Extension<auth::User>,
        user_service: State<user::Service>,
        event_service: State<event::Service>,
    ) -> sse::Sse<impl Stream<Item = crate::Result<sse::Event>>> {
        let auth_sub = auth_user.sub().clone();

        let stream = async_stream::try_stream! {
            let mut noti_stream = event_service
                .subscribe_noti(&Subject::Notifications(&auth_sub))
                .await?;

            let _osd = OnlineStatusDropper(&auth_sub, &user_service);
            let mut interval = time::interval(ONLINE_NOTI_INTERVAL);
            loop {
                tokio::select! {
                    noti = noti_stream.next() => {
                        if let Some(noti) = noti { yield sse::Event::from(noti) }
                    },
                    _ = interval.tick() => user_service.notify_online(&auth_sub).await
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

    struct OnlineStatusDropper<'a>(&'a user::Sub, &'a user::Service);

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

            sse::Event::default()
                .event(evt)
                .data(noti.render().into_string())
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
        State(talk_validator): State<talk::Validator>,
        State(event_service): State<event::Service>,
        State(message_service): State<message::Service>,
        State(talk_service): State<talk::Service>,
    ) -> crate::Result<Response> {
        debug!("Upgrading to WS for talk: {}", &talk_id);
        talk_validator.check_member(&talk_id, &auth_user).await?;

        let auth_sub = auth_user.sub().clone();
        Ok(ws.on_upgrade(move |socket| async {
            let (sender, recv) = socket.split();
            let close = Arc::new(Notify::new());

            tokio::spawn(send(
                auth_sub,
                talk_id.clone(),
                sender,
                event_service,
                message_service,
                talk_service,
                close.clone(),
            ));

            tokio::spawn(receive(talk_id, recv, close.clone()));
        }))
    }

    async fn send(
        auth_sub: user::Sub,
        talk_id: talk::Id,
        mut sender: SplitSink<WebSocket, ws::Message>,
        event_service: event::Service,
        message_service: message::Service,
        talk_service: talk::Service,
        close: Arc<Notify>,
    ) -> event::Result<()> {
        let mut msg_stream = event_service
            .subscribe_event(&Subject::Messages(&auth_sub, &talk_id))
            .await?;

        loop {
            tokio::select! {
                () = close.notified() => break,
                maybe_msg = msg_stream.next() => {
                    let Some(msg) = maybe_msg else { break };
                    let markup = msg.render().into_string();
                    if let Err(e) = sender.send(ws::Message::Binary(markup.into())).await {
                        error!("Failed to send event message to client: {e}");
                        break;
                    }

                    if let Message::New(msg) = msg {
                        let talk_id = msg.talk_id().clone();
                        match message_service.mark_as_seen(&auth_sub, &[msg]).await {
                            Ok(seen_qty) => {
                                if seen_qty == 0 {
                                    continue;
                                }

                                if let Err(e) = talk_service.mark_as_seen(&talk_id).await {
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
