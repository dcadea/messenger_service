use std::env;
use std::time::Duration;

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
    pub fn env() -> super::Result<Self> {
        let host = env::var("MONGO_HOST")?;
        let port = env::var("MONGO_PORT")?.parse()?;
        let db = env::var("MONGO_DB")?;
        Ok(Self { host, port, db })
    }
}

pub async fn init(config: &Config) -> super::Result<mongodb::Database> {
    let options = mongodb::options::ClientOptions::builder()
        .hosts(vec![mongodb::options::ServerAddress::Tcp {
            host: config.host.to_owned(),
            port: Some(config.port),
        }])
        .server_selection_timeout(Some(Duration::from_secs(2)))
        .connect_timeout(Some(Duration::from_secs(5)))
        .build();

    let db = mongodb::Client::with_options(options).map(|client| client.database(&config.db))?;

    Ok(db)
}

impl From<crate::user::Sub> for mongodb::bson::Bson {
    fn from(val: crate::user::Sub) -> Self {
        mongodb::bson::Bson::String(val.0)
    }
}
