use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};

use crate::integration::error::IntegrationError;

pub mod amqp;
pub mod error;
pub mod idp;
pub mod model;
pub mod mongo;
pub mod redis;

type Result<T> = std::result::Result<T, IntegrationError>;

#[derive(Clone)]
pub struct Config {
    pub socket: SocketAddr,

    pub redis: redis::Config,
    pub mongo: mongo::Config,
    pub amqp: amqp::Config,

    pub idp: idp::Config,
}

impl Default for Config {
    fn default() -> Self {
        dotenv().ok();

        let rust_log = env::var("RUST_LOG").unwrap_or("info".into());
        let level = LevelFilter::from_str(&rust_log).unwrap_or(LevelFilter::Info);
        CombinedLogger::init(vec![
            TermLogger::new(
                level,
                simplelog::Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
            WriteLogger::new(
                level,
                simplelog::Config::default(),
                File::create("api.log").expect("Failed to create log file"),
            ),
        ])
        .expect("Failed to initialize logger");

        let app_addr = env::var("APP_ADDR").unwrap_or("127.0.0.1".into());
        let app_port = env::var("APP_PORT").unwrap_or("8000".into());

        let socket = format!("{}:{}", app_addr, app_port)
            .parse()
            .expect("Failed to parse socket address");

        let idp_config = idp::Config::new(
            env::var("ISSUER").expect("ISSUER must be set"),
            env::var("AUDIENCE")
                .expect("AUDIENCE must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>(),
            env::var("REQUIRED_CLAIMS")
                .expect("REQUIRED_CLAIMS must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>(),
        );

        // FIXME: use testcontainers instead of env configs
        Self {
            socket,
            redis: redis::Config::env().unwrap_or_default(),
            mongo: mongo::Config::env().unwrap_or_default(),
            amqp: amqp::Config::env().unwrap_or_default(),
            idp: idp_config,
        }
    }
}

pub fn init_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(IntegrationError::from)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use log::LevelFilter;
    use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode};
    use testcontainers_modules::mongo::Mongo;
    use testcontainers_modules::rabbitmq::RabbitMq;
    use testcontainers_modules::redis::Redis;
    use testcontainers_modules::testcontainers::ContainerAsync;

    use crate::integration::{amqp, idp, mongo, redis, Config};

    impl Config {
        pub fn test() -> Self {
            let socket: SocketAddr = "127.0.0.1:8001".parse().unwrap();

            let level = LevelFilter::Debug;
            CombinedLogger::init(vec![TermLogger::new(
                level,
                simplelog::Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            )])
            .unwrap();

            // TODO: mock it
            let idp_config = idp::Config::new(
                String::from("https://dcadea.auth0.com/"),
                vec![String::from("https://dcadea.auth0.com/api/v1/")],
                vec![
                    String::from("iss"),
                    String::from("sub"),
                    String::from("aud"),
                    String::from("exp"),
                    String::from("permissions"),
                ],
            );

            Self {
                socket,
                redis: redis::Config::default(),
                mongo: mongo::Config::default(),
                amqp: amqp::Config::default(),
                idp: idp_config,
            }
        }

        pub async fn with_redis(&self, redis_container: ContainerAsync<Redis>) -> Self {
            let redis_config = redis::Config::new(
                redis_container.get_host().await.unwrap().to_string(),
                redis_container.get_host_port_ipv4(6379).await.unwrap(),
            );

            Self {
                socket: self.socket,
                redis: redis_config,
                mongo: self.mongo.clone(),
                amqp: self.amqp.clone(),
                idp: self.idp.clone(),
            }
        }

        pub async fn with_mongo(&self, mongo_container: ContainerAsync<Mongo>) -> Self {
            let mongo_config = mongo::Config::new(
                mongo_container.get_host().await.unwrap().to_string(),
                mongo_container.get_host_port_ipv4(27017).await.unwrap(),
                "test_messenger".to_string(),
            );

            Self {
                socket: self.socket,
                redis: self.redis.clone(),
                mongo: mongo_config,
                amqp: self.amqp.clone(),
                idp: self.idp.clone(),
            }
        }

        pub async fn with_amqp(&self, amqp_container: ContainerAsync<RabbitMq>) -> Self {
            let amqp_config = amqp::Config::new(
                amqp_container.get_host().await.unwrap().to_string(),
                amqp_container.get_host_port_ipv4(5672).await.unwrap(),
            );

            Self {
                socket: self.socket,
                redis: self.redis.clone(),
                mongo: self.mongo.clone(),
                amqp: amqp_config,
                idp: self.idp.clone(),
            }
        }
    }
}
