use log::debug;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::Reply;
use warp::reply::json;
use warp::ws::Message;

use crate::ws::client::client_connection;
use crate::ws::model::{Client, Clients, Event, RegisterRequest, RegisterResponse};

pub async fn register_handler(body: RegisterRequest, clients: Clients) -> crate::Result<impl Reply> {
    let user_id = body.user_id();
    let uuid = Uuid::new_v4().simple().to_string();

    register_client(uuid.clone(), user_id, clients).await;
    Ok(json(&RegisterResponse::new(format!("ws://127.0.0.1:8000/ws/{}", uuid))))
}

async fn register_client(id: String, user_id: usize, clients: Clients) {
    clients.write().await.insert(
        id,
        Client::new(user_id, vec![String::from("cats")], None),
    );
}

pub async fn unregister_handler(id: String, clients: Clients) -> crate::Result<impl Reply> {
    let mut clients_locked = clients.write().await;
    if let Some(client) = clients_locked.get(&id) {
        if let Some(sender) = &client.sender() {
            let _ = sender.send(Ok(Message::close()));
        }
    }
    clients_locked.remove(&id);
    debug!("{} disconnected", id);
    Ok(StatusCode::OK)
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients) -> crate::Result<impl Reply> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| client_connection(socket, id, clients, c))),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn publish_handler(body: Event, clients: Clients) -> crate::Result<impl Reply> {
    clients
        .write()
        .await
        .iter_mut()
        .filter(|(_, client)| match body.user_id() {
            Some(v) => client.user_id() == v,
            None => true,
        })
        .filter(|(_, client)| client.topics().contains(&body.topic().to_string()))
        .for_each(|(_, client)| {
            if let Some(sender) = &client.sender() {
                let _ = sender.send(Ok(Message::text(body.message())));
            }
        });

    Ok(StatusCode::OK)
}