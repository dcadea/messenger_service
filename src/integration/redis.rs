use std::env;

use crate::integration::Result;

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

pub async fn init(config: &Config) -> Result<redis::aio::ConnectionManager> {
    let redis_con =
        redis::Client::open(format!("redis://{}:{}", config.host.clone(), config.port))?
            .get_connection_manager()
            .await?;

    Ok(redis_con)
}

#[cfg(test)]
mod tests {
    use crate::integration::redis::Config;

    impl Config {
        pub fn new(host: String, port: u16) -> Self {
            Self { host, port }
        }
    }
}
