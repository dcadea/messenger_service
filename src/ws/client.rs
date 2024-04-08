use std::sync::Arc;

use crate::message::service::MessageService;
use futures::FutureExt;
use futures::StreamExt;
use lapin::options::BasicAckOptions;
use log::{debug, error};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

pub async fn client_connection(ws: WebSocket, topic: String, message_service: Arc<MessageService>) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("error sending websocket msg: {}", e);
        }
    }));

    debug!("{} connected", topic);

    let (mut consumer, channel) = message_service.consume(topic.as_str()).await.unwrap();
    let consumer_tag = consumer.tag();

    tokio::spawn(async move {
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.unwrap();
            let message = std::str::from_utf8(&delivery.data).unwrap();

            let _ = client_sender.send(Ok(Message::text(message)));

            // TODO: move to message service
            delivery.ack(BasicAckOptions::default()).await.unwrap();
        }
    });

    while let Some(result) = client_ws_rcv.next().await {
        let _ = match result {
            Ok(_) => continue,
            Err(e) => {
                error!(
                    "error receiving ws message for topic: {}): {}",
                    topic.clone(),
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
    debug!("{} disconnected", topic);
}
