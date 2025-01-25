use std::env;

use log::warn;

#[derive(Clone)]
pub struct Config {
    host: String,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 4222,
        }
    }
}

impl Config {
    pub fn env() -> Option<Self> {
        let host = env::var("NATS_HOST").ok();
        let port = env::var("NATS_PORT")
            .unwrap_or("4222".to_string())
            .parse()
            .ok();

        match (host, port) {
            (Some(host), Some(port)) => Some(Self { host, port }),
            _ => {
                warn!("NATS env is not configured");
                None
            }
        }
    }

    pub async fn connect(&self) -> async_nats::Client {
        match async_nats::connect(&format!("{}:{}", self.host, self.port)).await {
            Ok(con) => con,
            Err(e) => panic!("Failed to connect to NATS: {}", e),
        }
    }
}
