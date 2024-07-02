use std::env;

use lapin::uri::{AMQPAuthority, AMQPQueryString, AMQPScheme, AMQPUri, AMQPUserInfo};
use tokio::sync::RwLock;

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
            port: 5672,
        }
    }
}

impl Config {
    pub fn env() -> Result<Self> {
        let host = env::var("AMQP_HOST")?;
        let port = env::var("AMQP_PORT")?.parse()?;
        Ok(Self { host, port })
    }
}

pub async fn init(config: &Config) -> Result<RwLock<lapin::Connection>> {
    let amqp_uri = AMQPUri {
        scheme: AMQPScheme::AMQP,
        authority: AMQPAuthority {
            userinfo: AMQPUserInfo::default(),
            host: config.host.clone(),
            port: config.port,
        },
        vhost: "/".to_string(),
        query: AMQPQueryString::default(),
    };

    let con = lapin::Connection::connect_uri(amqp_uri, lapin::ConnectionProperties::default())
        .await
        .map(|con| RwLock::new(con))?;

    Ok(con)
}

#[cfg(test)]
pub mod tests {
    use testcontainers_modules::rabbitmq::RabbitMq;
    use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::testcontainers::runners::AsyncRunner;
    use crate::integration::amqp::Config;

    pub struct TestContainer {
        amqp: ContainerAsync<RabbitMq>,
        pub config: Config,
    }

    impl TestContainer {
        pub async fn init() -> Self {
            let amqp = RabbitMq::default()
                .with_container_name("awg_test_amqp")
                .start()
                .await
                .unwrap();

            let config = Config {
                host: amqp.get_host().await.unwrap().to_string(),
                port: amqp.get_host_port_ipv4(5672).await.unwrap(),
            };

            Self { amqp, config }
        }
    }
}
