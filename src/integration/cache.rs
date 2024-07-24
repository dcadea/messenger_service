use crate::integration;
use crate::integration::Result;
use std::env;

#[derive(Clone)]
pub struct Config {
    host: String,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 6379,
        }
    }
}

impl Config {
    pub fn env() -> Result<Self> {
        let host = env::var("REDIS_HOST")?;
        let port = env::var("REDIS_PORT")?.parse()?;
        Ok(Self { host, port })
    }
}

pub async fn init_client(config: &Config) -> Result<redis::Client> {
    redis::Client::open(format!("redis://{}:{}", &config.host, &config.port))
        .map_err(integration::Error::from)
}

pub async fn init(config: &Config) -> Result<redis::aio::ConnectionManager> {
    init_client(config)
        .await?
        .get_connection_manager()
        .await
        .map_err(integration::Error::from)
}
