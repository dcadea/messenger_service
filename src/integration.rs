use std::env;
use std::sync::Arc;
use std::time::Duration;

use dotenv::dotenv;
use tokio::sync::Mutex;

use crate::result::Result;

#[derive(Clone)]
pub struct Config {
    pub redis_host: String,
    pub redis_port: String,

    pub mongo_username: String,
    pub mongo_password: String,
    pub mongo_host: String,
    pub mongo_port: String,
    pub mongo_db: String,

    pub amqp_addr: String,

    pub issuer: String,
    pub jwks_url: String,
    pub userinfo_url: String,
    pub audience: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        dotenv().ok();

        Self {
            redis_host: env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            redis_port: env::var("REDIS_PORT").unwrap_or_else(|_| "6379".into()),

            mongo_username: env::var("MONGO_USERNAME").unwrap_or_else(|_| "root".into()),
            mongo_password: env::var("MONGO_PASSWORD").unwrap_or_else(|_| "example".into()),
            mongo_host: env::var("MONGO_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            mongo_port: env::var("MONGO_PORT").unwrap_or_else(|_| "27017".into()),
            mongo_db: env::var("MONGO_DB").unwrap_or_else(|_| "messenger".into()),

            amqp_addr: env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into()),

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
        }
    }
}

pub fn init_redis(config: &Config) -> Result<redis::Connection> {
    let host = config.redis_host.clone();
    let port = config.redis_port.clone();

    let con = redis::Client::open(format!("redis://{}:{}", host, port))?
        .get_connection_with_timeout(Duration::from_secs(2))?;

    Ok(con)
}

pub async fn init_mongodb(config: &Config) -> Result<mongodb::Database> {
    let username = config.mongo_username.clone();
    let password = config.mongo_password.clone();
    let host = config.mongo_host.clone();
    let port = config.mongo_port.clone();
    let database = config.mongo_db.clone();

    let connection_url = format!("mongodb://{}:{}@{}:{}", username, password, host, port);

    let mut mongo_client_options = mongodb::options::ClientOptions::parse(connection_url).await?;

    mongo_client_options.connect_timeout = Some(Duration::from_secs(5));
    mongo_client_options.server_selection_timeout = Some(Duration::from_secs(2));

    let client = mongodb::Client::with_options(mongo_client_options)?;

    Ok(client.database(&*database))
}

pub async fn init_rabbitmq(config: &Config) -> Result<Arc<Mutex<lapin::Connection>>> {
    let addr = config.amqp_addr.clone();

    let map = lapin::Connection::connect(&addr, lapin::ConnectionProperties::default())
        .await
        .map(|con| Arc::new(Mutex::new(con)))?;

    Ok(map)
}

pub fn init_http_client() -> Result<Arc<reqwest::Client>> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .map(Arc::new)
        .map(Ok)?
}
