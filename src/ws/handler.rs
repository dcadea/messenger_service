use std::sync::Arc;

use log::debug;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::json;
use warp::{Rejection, Reply};

use crate::message::service::MessageService;
use crate::ws::client::client_connection;
use crate::ws::model::{Event, RegisterResponse, TopicsRequest, WsClient};
use crate::ws::service::WsClientService;

type Result<T> = std::result::Result<T, Rejection>;

pub async fn register_handler(
    username: String,
    topics_request: TopicsRequest,
    ws_client_service: Arc<WsClientService>,
) -> Result<impl Reply> {
    let uuid = Uuid::new_v4().simple().to_string();

    ws_client_service
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
) -> Result<impl Reply> {
    ws_client_service.unregister_client(id.clone()).await;
    debug!("{} disconnected", id);
    Ok(StatusCode::OK)
}

pub async fn ws_handler(
    ws: warp::ws::Ws,
    id: String,
    ws_client_service: Arc<WsClientService>,
    message_service: Arc<MessageService>,
) -> Result<impl Reply> {
    let ws_client = ws_client_service.get_client(id.clone()).await;
    match ws_client {
        Some(wsc) => Ok(ws.on_upgrade(move |socket| {
            client_connection(socket, id, wsc, ws_client_service, message_service)
        })),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn publish_handler(
    body: Event,
    message_service: Arc<MessageService>,
) -> Result<impl Reply> {
    message_service.publish(body).await;
    Ok(StatusCode::OK)
}
