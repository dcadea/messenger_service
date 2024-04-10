use futures::StreamExt;
use lapin::options::BasicAckOptions;
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::message::model::MessageRequest;
use warp::http::StatusCode;
use warp::ws::{Message, WebSocket};
use warp::{Rejection, Reply};

use futures::FutureExt;

use crate::message::service::MessageService;
use crate::user::repository::UserRepository;

type Result<T> = std::result::Result<T, Rejection>;

pub async fn ws_handler(
    ws: warp::ws::Ws,
    recipient: String,
    user_repository: Arc<UserRepository>,
    message_service: Arc<MessageService>,
) -> Result<impl Reply> {
    match user_repository.find_one(recipient.as_str()).await {
        Some(_) => {
            Ok(ws.on_upgrade(move |socket| client_connection(socket, recipient, message_service)))
        }
        None => Err(warp::reject::not_found()),
    }
}

pub async fn messages_handler(
    request: MessageRequest,
    message_service: Arc<MessageService>,
) -> Result<impl Reply> {
    message_service.send(request).await;
    Ok(StatusCode::OK)
}

pub async fn client_connection(
    ws: WebSocket,
    recipient: String,
    message_service: Arc<MessageService>,
) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("error sending websocket msg: {}", e);
        }
    }));

    debug!("{} connected", recipient);

    let (mut consumer, channel) = message_service.consume(recipient.as_str()).await.unwrap();
    let consumer_tag = consumer.tag();

    tokio::spawn(async move {
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.unwrap();
            let message_json = std::str::from_utf8(&delivery.data).unwrap();

            let _ = client_sender.send(Ok(Message::text(message_json)));

            // TODO: move to message service
            delivery.ack(BasicAckOptions::default()).await.unwrap();
        }
    });

    while let Some(result) = client_ws_rcv.next().await {
        let _ = match result {
            Ok(_) => continue,
            Err(e) => {
                error!(
                    "error receiving ws message for recipient: {}): {}",
                    recipient.clone(),
                    e
                );
                break;
            }
        };
    }

    message_service
        .close_consumer(consumer_tag.as_str(), channel)
        .await
        .unwrap();
    debug!("{} disconnected", recipient);
}
