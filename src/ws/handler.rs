use std::sync::Arc;

use log::debug;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::json;
use warp::ws::Message;
use warp::Reply;

use crate::ws::client::client_connection;
use crate::ws::model::{Event, RegisterResponse, TopicsRequest, WsClient};
use crate::ws::service::WsClientService;

pub async fn register_handler(
    username: String,
    topics_request: TopicsRequest,
    ws_client_service: Arc<WsClientService>,
) -> crate::Result<impl Reply> {
    let uuid = Uuid::new_v4().simple().to_string();

    Arc::clone(&ws_client_service)
        .register_client(
            uuid.clone(),
            WsClient::new(username, topics_request.topics().clone(), None),
        )
        .await;

    Ok(json(&RegisterResponse::new(format!(
        "ws://127.0.0.1:8000/ws/{}",
        uuid
    ))))
}

pub async fn unregister_handler(
    id: String,
    ws_client_service: Arc<WsClientService>,
) -> crate::Result<impl Reply> {
    Arc::clone(&ws_client_service)
        .unregister_client(id.clone())
        .await;
    debug!("{} disconnected", id);
    Ok(StatusCode::OK)
}

pub async fn ws_handler(
    ws: warp::ws::Ws,
    id: String,
    ws_client_service: Arc<WsClientService>,
) -> crate::Result<impl Reply> {
    let ws_client = ws_client_service.get_client(id.clone()).await;
    match ws_client {
        Some(wsc) => Ok(ws.on_upgrade(move |socket| {
            client_connection(socket, id, wsc, Arc::clone(&ws_client_service))
        })),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn publish_handler(
    body: Event,
    ws_client_service: Arc<WsClientService>,
) -> crate::Result<impl Reply> {
    ws_client_service
        .get_clients()
        .await
        .read()
        .await
        .iter()
        .filter(|(_, ws_client)| match body.username() {
            Some(v) => ws_client.username() == v,
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
