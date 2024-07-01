use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};

use crate::integration::error::IntegrationError;

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

pub mod redis {
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
}

pub mod mongo {
    use std::env;
    use std::time::Duration;

    use crate::integration::Result;

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
        pub fn env() -> Result<Self> {
            let host = env::var("MONGO_HOST")?;
            let port = env::var("MONGO_PORT")?.parse()?;
            let db = env::var("MONGO_DB")?;
            Ok(Self { host, port, db })
        }
    }

    pub async fn init(config: &Config) -> Result<mongodb::Database> {
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
