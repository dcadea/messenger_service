use std::sync::Arc;

use log::debug;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::Reply;
use warp::reply::json;
use warp::ws::Message;

use crate::ws::client::client_connection;
use crate::ws::model::{Event, RegisterResponse, WsClient, WsClients};
use crate::ws::service::WsClientService;

pub async fn register_handler(user_id: usize, ws_client_service: Arc<WsClientService>) -> crate::Result<impl Reply> {
    let uuid = Uuid::new_v4().simple().to_string();

    register_ws_client(uuid.clone(), user_id, ws_client_service).await;
    Ok(json(&RegisterResponse::new(format!("ws://127.0.0.1:8000/ws/{}", uuid))))
}

async fn register_ws_client(id: String, user_id: usize, ws_client_service: Arc<WsClientService>) {
    Arc::clone(&ws_client_service)
        .register_client(
            id.clone(),
            WsClient::new(user_id, vec![String::from("cats")], None),
        ).await;
}

pub async fn unregister_handler(id: String, ws_client_service: Arc<WsClientService>) -> crate::Result<impl Reply> {
    Arc::clone(&ws_client_service).unregister_client(id.clone()).await;
    debug!("{} disconnected", id);
    Ok(StatusCode::OK)
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, ws_clients: WsClients, ws_client_service: Arc<WsClientService>) -> crate::Result<impl Reply> {
    let ws_client_service = Arc::clone(&ws_client_service);

    let ws_client = ws_client_service.get_client(id.clone()).await;
    match ws_client {
        Some(wsc) => Ok(ws.on_upgrade(move |socket| client_connection(socket, id, ws_clients, wsc))),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn publish_handler(body: Event, ws_clients: WsClients) -> crate::Result<impl Reply> {
    ws_clients
        .write()
        .await
        .iter_mut()
        .filter(|(_, ws_client)| match body.user_id() {
            Some(v) => ws_client.user_id() == v,
            None => true,
        })
        .filter(|(_, ws_client)| ws_client.topics().contains(&body.topic().to_string()))
        .for_each(|(_, ws_client)| {
            if let Some(sender) = &ws_client.sender() {
                let _ = sender.send(Ok(Message::text(body.message())));
            }
        });

    Ok(StatusCode::OK)
}