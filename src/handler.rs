use std::future::Future;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::Reply;
use warp::reply::json;
use warp::ws::Message;
use crate::{Client, Clients, Event, RegisterRequest, RegisterResponse, Result, ws};

pub async fn register_handler(body: RegisterRequest, clients: Clients) -> Result<impl Reply> {
    let user_id = body.user_id;
    let uuid = Uuid::new_v4().simple().to_string();

    register_client(uuid.clone(), user_id, clients).await;
    Ok(json(&RegisterResponse {
        url: format!("ws://127.0.0.1:8000/ws/{}", uuid),
    }))
}

async fn register_client(id: String, user_id: usize, clients: Clients) {
    clients.write().await.insert(
        id,
        Client {
            user_id,
            topics: vec![String::from("cats")],
            sender: None,
        },
    );
}

pub async fn unregister_handler(id: String, clients: Clients) -> Result<impl Reply> {
    let mut clients_locked = clients.write().await;
    if let Some(client) = clients_locked.get(&id) {
        if let Some(sender) = &client.sender {
            let _ = sender.send(Ok(Message::close()));
        }
    }
    clients_locked.remove(&id);
    println!("{} disconnected", id);
    Ok(StatusCode::OK)
}

pub fn health_handler() -> impl Future<Output=Result<impl Reply>> {
    futures::future::ready(Ok(StatusCode::OK))
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients) -> Result<impl Reply> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, id, clients, c))),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn publish_handler(body: Event, clients: Clients) -> Result<impl Reply> {
    clients
        .write()
        .await
        .iter_mut()
        .filter(|(_, client)| match body.user_id {
            Some(v) => client.user_id == v,
            None => true,
        })
        .filter(|(_, client)| client.topics.contains(&body.topic))
        .for_each(|(_, client)| {
            if let Some(sender) = &client.sender {
                let _ = sender.send(Ok(Message::text(body.message.clone())));
            }
        });

    Ok(StatusCode::OK)
}

