use std::time::Duration;
use std::{env, str::FromStr};

use log::warn;
use mongodb::bson::{doc, oid};

use crate::{chat, message, user};

#[derive(Clone)]
pub struct Config {
    host: String,
    port: u16,
    db: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 27017,
            db: String::from("messenger"),
        }
    }
}

impl Config {
    pub fn env() -> Option<Self> {
        let host = env::var("MONGO_HOST").ok();
        let port = env::var("MONGO_PORT")
            .unwrap_or("27017".to_string())
            .parse()
            .ok();
        let db = env::var("MONGO_DB").unwrap_or_else(|_e| String::from("messenger"));

        match (host, port) {
            (Some(host), Some(port)) => Some(Self { host, port, db }),
            _ => {
                warn!("MONGO env is not configured");
                None
            }
        }
    }

    pub fn connect(&self) -> mongodb::Database {
        let options = mongodb::options::ClientOptions::builder()
            .hosts(vec![mongodb::options::ServerAddress::Tcp {
                host: self.host.to_owned(),
                port: Some(self.port),
            }])
            .server_selection_timeout(Some(Duration::from_secs(2)))
            .connect_timeout(Some(Duration::from_secs(5)))
            .build();

        match mongodb::Client::with_options(options).map(|client| client.database(&self.db)) {
            Ok(db) => db,
            Err(e) => panic!("Failed to connect to MongoDB: {}", e),
        }
    }
}

impl From<chat::Id> for mongodb::bson::Bson {
    fn from(val: chat::Id) -> Self {
        match oid::ObjectId::from_str(&val.0) {
            Ok(oid) => mongodb::bson::Bson::ObjectId(oid),
            Err(_) => mongodb::bson::Bson::String(val.0.clone()),
        }
    }
}

impl From<message::Id> for mongodb::bson::Bson {
    fn from(val: message::Id) -> Self {
        match oid::ObjectId::from_str(&val.0) {
            Ok(oid) => mongodb::bson::Bson::ObjectId(oid),
            Err(_) => mongodb::bson::Bson::String(val.0.clone()),
        }
    }
}

impl From<user::Id> for mongodb::bson::Bson {
    fn from(val: user::Id) -> Self {
        match oid::ObjectId::from_str(&val.0) {
            Ok(oid) => mongodb::bson::Bson::ObjectId(oid),
            Err(_) => mongodb::bson::Bson::String(val.0.clone()),
        }
    }
}

impl From<user::Sub> for mongodb::bson::Bson {
    fn from(val: user::Sub) -> Self {
        mongodb::bson::Bson::String(val.0)
    }
}

impl From<message::model::LastMessage> for mongodb::bson::Bson {
    fn from(lm: message::model::LastMessage) -> Self {
        Self::Document(doc! {
            "id": lm.id,
            "text": lm.text,
            "recipient": lm.recipient,
            "timestamp": lm.timestamp,
            "seen": lm.seen
        })
    }
}
