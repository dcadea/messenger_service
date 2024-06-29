use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};
use tokio::sync::RwLock;

use crate::integration::error::IntegrationError;

pub mod error;
pub mod model;

type Result<T> = std::result::Result<T, IntegrationError>;

#[derive(Clone)]
pub struct Config {
    pub socket: SocketAddr,
    pub redis_host: String,
    pub redis_port: String,

    pub mongo_username: String,
    pub mongo_password: String,
    pub mongo_host: String,
    pub mongo_port: String,
    pub mongo_db: String,

    pub amqp_host: String,
    pub amqp_port: String,

    pub issuer: String,
    pub jwks_url: String,
    pub userinfo_url: String,
    pub audience: Vec<String>,
    pub required_claims: Vec<String>,
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

        Self {
            socket,
            redis_host: env::var("REDIS_HOST").unwrap_or("127.0.0.1".into()),
            redis_port: env::var("REDIS_PORT").unwrap_or("6379".into()),

            mongo_username: env::var("MONGO_USERNAME").unwrap_or("root".into()),
            mongo_password: env::var("MONGO_PASSWORD").unwrap_or("example".into()),
            mongo_host: env::var("MONGO_HOST").unwrap_or("127.0.0.1".into()),
            mongo_port: env::var("MONGO_PORT").unwrap_or("27017".into()),
            mongo_db: env::var("MONGO_DB").unwrap_or("messenger".into()),

            amqp_host: env::var("AMQP_HOST").unwrap_or("127.0.0.1".into()),
            amqp_port: env::var("AMQP_PORT").unwrap_or("5672".into()),

            issuer: env::var("ISSUER").expect("ISSUER must be set"),
            jwks_url: env::var("ISSUER")
                .map(|iss| format!("{}.well-known/jwks.json", iss))
                .expect("ISSUER must be set"),
            userinfo_url: env::var("ISSUER")
                .map(|iss| format!("{}userinfo", iss))
                .expect("ISSUER must be set"),
            audience: env::var("AUDIENCE")
                .expect("AUDIENCE must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>(),
            required_claims: env::var("REQUIRED_CLAIMS")
                .expect("REQUIRED_CLAIMS must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>(),
        }
    }
}

pub async fn init_redis(config: &Config) -> Result<redis::aio::MultiplexedConnection> {
    let host = config.redis_host.clone();
    let port = config.redis_port.clone();

    redis::Client::open(format!("redis://{}:{}", host, port))?
        .get_multiplexed_async_connection_with_timeouts(
            Duration::from_secs(2),
            Duration::from_secs(5),
        )
        .await
        .map_err(IntegrationError::from)
}

pub async fn init_mongodb(config: &Config) -> Result<mongodb::Database> {
    let username = config.mongo_username.clone();
    let password = config.mongo_password.clone();
    let host = config.mongo_host.clone();
    let port = config.mongo_port.clone();
    let database = config.mongo_db.clone();

    let connection_url = format!("mongodb://{}:{}@{}:{}", username, password, host, port);

    let mut options = mongodb::options::ClientOptions::parse(connection_url).await?;

    options.connect_timeout = Some(Duration::from_secs(5));
    options.server_selection_timeout = Some(Duration::from_secs(2));

    mongodb::Client::with_options(options)
        .map(|client| client.database(&database))
        .map_err(IntegrationError::from)
}

pub async fn init_rabbitmq(config: &Config) -> Result<RwLock<lapin::Connection>> {
    let host = config.amqp_host.clone();
    let port = config.amqp_port.clone();
    let addr = format!("amqp://{}:{}/%2f", host, port);

    lapin::Connection::connect(&addr, lapin::ConnectionProperties::default())
        .await
        .map(|con| RwLock::new(con))
        .map_err(IntegrationError::from)
}

pub fn init_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(IntegrationError::from)
}
