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
pub mod tests {
    use testcontainers_modules::redis::Redis;
    use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    use crate::integration::redis::Config;

    pub struct TestContainer {
        redis: ContainerAsync<Redis>,
        pub config: Config,
    }

    impl TestContainer {
        pub async fn init() -> Self {
            let redis = Redis::default()
                .with_container_name("awg_test_redis")
                .start()
                .await
                .unwrap();

            let config = Config {
                host: redis.get_host().await.unwrap().to_string(),
                port: redis.get_host_port_ipv4(6379).await.unwrap(),
            };

            Self { redis, config }
        }
    }
}
