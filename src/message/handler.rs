use std::sync::Arc;

use futures::FutureExt;
use futures::StreamExt;
use log::{debug, error};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::http::StatusCode;
use warp::reply::{json, with_status};
use warp::ws::{Message, WebSocket};
use warp::{Rejection, Reply};

use crate::error::model::ApiError;
use crate::message::model::MessageRequest;
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
    match message_service.publish_for_recipient(request).await {
        Ok(response) => Ok(with_status(json(&response), StatusCode::CREATED)),
        Err(e) => {
            error!("Failed to send a message: {}", e);
            Ok(with_status(
                json(&ApiError::new("Failed to send a message")),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn client_connection(ws: WebSocket, recipient: String, message_service: Arc<MessageService>) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_sender = Arc::new(Mutex::new(client_sender));

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("error sending websocket msg: {}", e);
        }
    }));

    debug!("{} connected", recipient);

    let (consumer_tag, channel, messages_stream) = match message_service.read(&recipient).await {
        Ok(binding) => binding,
        Err(e) => {
            error!("Failed to read messages: {}", e);
            return;
        }
    };

    let client_sender_clone = client_sender.clone();
    let message_service_clone = message_service.clone();
    tokio::spawn(messages_stream.for_each(move |data| {
        let client_sender = client_sender_clone.clone();
        let message_service = message_service_clone.clone();
        async move {
            match data {
                Ok(data) => {
                    let _ = client_sender
                        .lock()
                        .await
                        .send(Ok(Message::text(data.clone())));
                    if let Err(e) = message_service.publish_for_storage(data).await {
                        error!("Failed to store message: {}", e);
                    }
                }
                Err(e) => error!("Failed to read message: {}", e),
            }
        }
    }));

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

    if let Err(e) = message_service.close_consumer(consumer_tag, channel).await {
        error!("Failed to close consumer: {}", e);
    };

    debug!("{} disconnected", recipient);
}
