use std::sync::Arc;

use redis::{Commands, Connection};
use tokio::sync::Mutex;

use crate::ws::model::WsClient;

pub struct WsClientService {
    con: Arc<Mutex<Connection>>,
}

pub fn init_ws_client_service(con: Arc<Mutex<Connection>>) -> WsClientService {
    WsClientService {
        con,
    }
}

impl WsClientService {
    pub async fn register_client(&self, id: String, ws_client: WsClient) {
        let ws_client_json = serde_json::to_string(&ws_client).unwrap();
        let con = Arc::clone(&self.con);
        let mut con = con.lock().await;
        let _: () = con.set(format!("ws_client:{}", &id), ws_client_json).unwrap();
    }

    pub async fn unregister_client(&self, id: String) {
        let con = Arc::clone(&self.con);
        let mut con = con.lock().await;
        let _: () = con.del(format!("ws_client:{}", &id)).unwrap();
    }
    // TODO
    // pub async fn get_client(&self, id: String) -> Option<WsClient> {
    //     let result: Option<String> = redis::cmd("HGET")
    //         .arg("ws_clients")
    //         .arg(id)
    //         .query_async(&self.con)
    //         .await
    //         .unwrap();
    //     match result {
    //         Some(v) => Some(serde_json::from_str(&v).unwrap()),
    //         None => None,
    //     }
    // }
    //
    // pub async fn publish_message(&self, body: Event) {
    //     let clients: Vec<WsClient> = redis::cmd("HVALS")
    //         .arg("ws_clients")
    //         .query_async(&self.con)
    //         .await
    //         .unwrap();
    //     clients
    //         .iter()
    //         .filter(|ws_client| match body.user_id() {
    //             Some(v) => ws_client.user_id() == v,
    //             None => true,
    //         })
    //         .filter(|ws_client| ws_client.topics().contains(&body.topic().to_string()))
    //         .for_each(|ws_client| {
    //             if let Some(sender) = &ws_client.sender() {
    //                 let _ = sender.send(Ok(Message::text(body.message())));
    //             }
    //         });
    // }
}