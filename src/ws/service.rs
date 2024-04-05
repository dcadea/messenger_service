use std::collections::HashMap;
use std::sync::Arc;

use redis::{Commands, Connection, RedisResult};
use tokio::sync::{Mutex, RwLock};

use crate::ws::model::WsClient;

type WsClients = Arc<RwLock<HashMap<String, WsClient>>>;

pub struct WsClientService {
    con: Arc<Mutex<Connection>>,
    clients: WsClients,
}

pub fn init_ws_client_service(con: Arc<Mutex<Connection>>) -> WsClientService {
    WsClientService {
        con,
        clients: Arc::new(RwLock::new(HashMap::new())),
    }
}

impl WsClientService {
    pub async fn register_client(&self, id: String, ws_client: WsClient) {
        let ws_client_json = serde_json::to_string(&ws_client).unwrap();
        let con = Arc::clone(&self.con);
        let mut con = con.lock().await;
        let _: () = con
            .set(format!("ws_client:{}", &id), ws_client_json)
            .unwrap();
    }

    pub async fn unregister_client(&self, id: String) {
        let con = Arc::clone(&self.con);
        let mut con = con.lock().await;
        let _: () = con.del(format!("ws_client:{}", &id)).unwrap();
        self.clients.write().await.remove(&id);
    }

    pub async fn get_client(&self, id: String) -> Option<WsClient> {
        let con = Arc::clone(&self.con);
        let mut con = con.lock().await;

        let ws_client_json: RedisResult<String> = con.get(format!("ws_client:{}", &id));

        ws_client_json
            .map(|json| serde_json::from_str(&json).ok())
            .unwrap_or(None)
    }

    pub async fn sync_client(&self, id: String, ws_client: WsClient) {
        self.clients.write().await.insert(id, ws_client);
    }

    pub async fn get_clients(&self) -> WsClients {
        Arc::clone(&self.clients)
    }
}
