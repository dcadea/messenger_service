use std::sync::Arc;

use warp::http::StatusCode;
use warp::{Rejection, Reply};

use crate::message::service::MessageService;
use crate::user::repository::UserRepository;
use crate::ws::client::client_connection;
use crate::ws::model::Event;

type Result<T> = std::result::Result<T, Rejection>;

pub async fn ws_handler(
    ws: warp::ws::Ws,
    topic: String,
    user_repository: Arc<UserRepository>,
    message_service: Arc<MessageService>,
) -> Result<impl Reply> {
    match user_repository.find_one(topic.as_str()).await {
        Some(_) => {
            Ok(ws.on_upgrade(move |socket| client_connection(socket, topic, message_service)))
        }
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
