use std::sync::Arc;

use futures::FutureExt;
use futures::StreamExt;
use log::{debug, error};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::WebSocket;

use crate::ws::model::WsClient;
use crate::ws::service::WsClientService;

pub async fn client_connection(
    ws: WebSocket,
    id: String,
    mut ws_client: WsClient,
    ws_client_service: Arc<WsClientService>,
) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("error sending websocket msg: {}", e);
        }
    }));

    ws_client.set_sender(client_sender);
    ws_client_service.sync_client(id.clone(), ws_client).await;

    debug!("{} connected", id);

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
