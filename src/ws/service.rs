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

    pub async fn get_client(&self, id: String) -> Option<WsClient> {
        let con = Arc::clone(&self.con);
        let mut con = con.lock().await;
        let ws_client_json: String = con.get(format!("ws_client:{}", &id)).unwrap();
        match serde_json::from_str(&ws_client_json) {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }
}