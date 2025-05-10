use std::time::Duration;
use std::{env, str::FromStr};

use log::warn;
use mongodb::bson::{doc, oid};

use crate::{contact, message, talk, user};

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
            .unwrap_or_else(|_| "27017".to_string())
            .parse()
            .ok();
        let db = env::var("MONGO_DB").unwrap_or_else(|_e| String::from("messenger"));

        if let (Some(host), Some(port)) = (host, port) {
            Some(Self { host, port, db })
        } else {
            warn!("MONGO env is not configured");
            None
        }
    }

    pub fn connect(&self) -> mongodb::Database {
        let options = mongodb::options::ClientOptions::builder()
            .hosts(vec![mongodb::options::ServerAddress::Tcp {
                host: self.host.clone(),
                port: Some(self.port),
            }])
            .server_selection_timeout(Some(Duration::from_secs(2)))
            .connect_timeout(Some(Duration::from_secs(5)))
            .build();

        match mongodb::Client::with_options(options).map(|client| client.database(&self.db)) {
            Ok(db) => db,
            Err(e) => panic!("Failed to connect to MongoDB: {e}"),
        }
    }
}

#[cfg(test)]
use testcontainers_modules::mongo::Mongo;

#[cfg(test)]
impl Config {
    pub async fn test(
        node: &testcontainers_modules::testcontainers::ContainerAsync<Mongo>,
    ) -> Self {
        let host = node.get_host().await.unwrap();
        let port = node.get_host_port_ipv4(27017).await.unwrap();
        Self {
            host: host.to_string(),
            port,
            db: "test".into(),
        }
    }
}

impl From<contact::Id> for mongodb::bson::Bson {
    fn from(val: contact::Id) -> Self {
        match oid::ObjectId::from_str(&val.0) {
            Ok(oid) => Self::ObjectId(oid),
            Err(_) => Self::String(val.0.clone()),
        }
    }
}

impl From<contact::Status> for mongodb::bson::Bson {
    fn from(val: contact::Status) -> Self {
        let doc = match val {
            contact::Status::Pending { initiator } => {
                doc! { "indicator": "pending", "initiator": initiator }
            }
            contact::Status::Accepted => doc! {"indicator": "accepted"},
            contact::Status::Rejected => doc! {"indicator": "rejected"},
            contact::Status::Blocked { initiator } => {
                doc! {"indicator": "blocked", "initiator": initiator}
            }
        };

        Self::Document(doc)
    }
}

impl From<talk::Id> for mongodb::bson::Bson {
    fn from(val: talk::Id) -> Self {
        match oid::ObjectId::from_str(&val.0) {
            Ok(oid) => Self::ObjectId(oid),
            Err(_) => Self::String(val.0.clone()),
        }
    }
}

impl From<message::Id> for mongodb::bson::Bson {
    fn from(val: message::Id) -> Self {
        match oid::ObjectId::from_str(&val.0) {
            Ok(oid) => Self::ObjectId(oid),
            Err(_) => Self::String(val.0.clone()),
        }
    }
}

impl From<user::Id> for mongodb::bson::Bson {
    fn from(val: user::Id) -> Self {
        match oid::ObjectId::from_str(&val.0) {
            Ok(oid) => Self::ObjectId(oid),
            Err(_) => Self::String(val.0.clone()),
        }
    }
}

impl From<user::Sub> for mongodb::bson::Bson {
    fn from(val: user::Sub) -> Self {
        Self::String(val.0.to_string())
    }
}

impl From<user::Nickname> for mongodb::bson::Bson {
    fn from(val: user::Nickname) -> Self {
        Self::String(val.0.to_string())
    }
}

impl From<message::model::LastMessage> for mongodb::bson::Bson {
    fn from(lm: message::model::LastMessage) -> Self {
        Self::Document(doc! {
            "id": lm.id(),
            "text": lm.text(),
            "owner": lm.owner(),
            "timestamp": lm.timestamp(),
            "seen": lm.seen()
        })
    }
}
