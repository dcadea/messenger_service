use std::env;
use std::time::Duration;

use crate::user;

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
    pub fn env() -> anyhow::Result<Self> {
        let host = env::var("MONGO_HOST")?;
        let port = env::var("MONGO_PORT")?.parse()?;
        let db = env::var("MONGO_DB")?;
        Ok(Self { host, port, db })
    }
}

pub fn init(config: &Config) -> mongodb::Database {
    let options = mongodb::options::ClientOptions::builder()
        .hosts(vec![mongodb::options::ServerAddress::Tcp {
            host: config.host.to_owned(),
            port: Some(config.port),
        }])
        .server_selection_timeout(Some(Duration::from_secs(2)))
        .connect_timeout(Some(Duration::from_secs(5)))
        .build();

    match mongodb::Client::with_options(options).map(|client| client.database(&config.db)) {
        Ok(db) => db,
        Err(e) => panic!("Failed to connect to MongoDB: {}", e),
    }
}

impl From<user::Sub> for mongodb::bson::Bson {
    fn from(val: user::Sub) -> Self {
        mongodb::bson::Bson::String(val.0)
    }
}
