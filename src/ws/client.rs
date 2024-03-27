use futures::FutureExt;
use futures::StreamExt;
use log::{debug, error};
use serde_json::from_str;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use crate::ws::model::{WsClient, WsClients, TopicsRequest};

pub async fn client_connection(ws: WebSocket, id: String, ws_clients: WsClients, mut ws_client: WsClient) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("error sending websocket msg: {}", e);
        }
    }));

    ws_client.set_sender(client_sender);
    ws_clients.write().await.insert(id.clone(), ws_client);

    debug!("{} connected", id);

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };
        client_msg(&id, msg, &ws_clients).await;
    }

    ws_clients.write().await.remove(&id);
    debug!("{} disconnected", id);
}

async fn client_msg(id: &str, msg: Message, ws_clients: &WsClients) {
    debug!("received message from {}: {:?}", id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    if message == "ping" || message == "ping\n" {
        return;
    }

    let topics_req: TopicsRequest = match from_str(&message) {
        Ok(v) => v,
        Err(e) => {
            error!("error while parsing message to topics request: {}", e);
            return;
        }
    };

    let mut locked = ws_clients.write().await;
    match locked.get_mut(id) {
        Some(v) => {
            v.set_topics(topics_req.topics().clone());
        }
        None => return,
    };
}
