use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use crate::integration::error::IntegrationError;
use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};

pub mod error;
pub mod model;

#[cfg(test)]
mod tests;

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

        Self {
            socket,
            redis: redis::Config::default(),
            mongo: mongo::Config::default(),
            amqp: amqp::Config::default(),
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

pub mod redis {
    use std::time::Duration;

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

            // TODO
            // redis_host: env::var("REDIS_HOST").unwrap_or("127.0.0.1".into()),
            // redis_port: env::var("REDIS_PORT").unwrap_or("6379".into()),
        }
    }

    pub async fn init(
        config: &Config,
    ) -> crate::integration::Result<redis::aio::MultiplexedConnection> {
        let redis_con =
            redis::Client::open(format!("redis://{}:{}", config.host.clone(), config.port))?
                .get_multiplexed_async_connection_with_timeouts(
                    Duration::from_secs(2),
                    Duration::from_secs(5),
                )
                .await?;

        Ok(redis_con)
    }
}

pub mod mongo {
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

        // TODO
        // mongo_username: env::var("MONGO_USERNAME").unwrap_or("root".into()),
        // mongo_password: env::var("MONGO_PASSWORD").unwrap_or("example".into()),
        // mongo_host: env::var("MONGO_HOST").unwrap_or("127.0.0.1".into()),
        // mongo_port: env::var("MONGO_PORT").unwrap_or("27017".into()),
        // mongo_db: env::var("MONGO_DB").unwrap_or("messenger".into()),
    }

    pub async fn init(config: &Config) -> crate::integration::Result<mongodb::Database> {
        let options = mongodb::options::ClientOptions::builder()
            .hosts(vec![mongodb::options::ServerAddress::Tcp {
                host: config.host.clone(),
                port: Some(config.port),
            }])
            .server_selection_timeout(Some(Duration::from_secs(2)))
            .connect_timeout(Some(Duration::from_secs(5)))
            .build();

        let db =
            mongodb::Client::with_options(options).map(|client| client.database(&config.db))?;

        Ok(db)
    }
}

pub mod amqp {
    use lapin::uri::{AMQPAuthority, AMQPQueryString, AMQPScheme, AMQPUri, AMQPUserInfo};
    use tokio::sync::RwLock;

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

        // TODO
        // amqp_host: env::var("AMQP_HOST").unwrap_or("127.0.0.1".into()),
        // amqp_port: env::var("AMQP_PORT").unwrap_or("5672".into()),
    }

    pub async fn init(config: &Config) -> crate::integration::Result<RwLock<lapin::Connection>> {
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
}

pub mod idp {
    #[derive(Clone)]
    pub struct Config {
        pub issuer: String,
        pub jwks_url: String,
        pub userinfo_url: String,
        pub audience: Vec<String>,
        pub required_claims: Vec<String>,
    }

    impl Config {
        pub fn new(issuer: String, audience: Vec<String>, required_claims: Vec<String>) -> Self {
            Self {
                issuer: issuer.clone(),
                jwks_url: format!("{}.well-known/jwks.json", issuer),
                userinfo_url: format!("{}userinfo", issuer),
                audience,
                required_claims,
            }
        }
    }
}
