use std::sync::Arc;

use crate::message::service::MessageService;
use futures::FutureExt;
use futures::StreamExt;
use lapin::options::BasicAckOptions;
use log::{debug, error};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use crate::ws::model::WsClient;
use crate::ws::service::WsClientService;

pub async fn client_connection(
    ws: WebSocket,
    id: String,
    ws_client: WsClient,
    ws_client_service: Arc<WsClientService>,
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

    // ws_client.set_sender(client_sender);
    let topics = ws_client.topics();
    ws_client_service.sync_client(id.clone(), ws_client).await;

    debug!("{} connected", id);

    let client_sender = Arc::new(client_sender);
    // if let Some(sender) = client_sender {
    for topic in topics {
        let mut consumer = message_service.consume(topic.as_str()).await.unwrap();
        let client_sender = Arc::clone(&client_sender);
        tokio::spawn(async move {
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery.unwrap();
                let message = std::str::from_utf8(&delivery.data).unwrap();

                let _ = Arc::clone(&client_sender).send(Ok(Message::text(message)));

                // TODO: move to message service
                delivery.ack(BasicAckOptions::default()).await.unwrap();
            }
        });
    }

    // }

    while let Some(result) = client_ws_rcv.next().await {
        let _ = match result {
            Ok(_) => continue,
            Err(e) => {
                error!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };
    }

    ws_client_service.unregister_client(id.clone()).await;

    debug!("{} disconnected", id);
}
