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
            port: 4222,
        }
    }
}

impl Config {
    pub fn env() -> anyhow::Result<Self> {
        let host = env::var("NATS_HOST")?;
        let port = env::var("NATS_PORT")
            .unwrap_or("4222".to_string())
            .parse()?;
        Ok(Self { host, port })
    }

    pub async fn connect(&self) -> async_nats::Client {
        match async_nats::connect(&format!("{}:{}", self.host, self.port)).await {
            Ok(con) => con,
            Err(e) => panic!("Failed to connect to NATS: {}", e),
        }
    }
}
