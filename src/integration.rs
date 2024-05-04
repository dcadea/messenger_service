use std::env;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::result::Result;

pub fn init_redis() -> Result<redis::Connection> {
    let host = env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = env::var("REDIS_PORT").unwrap_or_else(|_| "6379".into());

    let con = redis::Client::open(format!("redis://{}:{}", host, port))?
        .get_connection_with_timeout(Duration::from_secs(2))?;

    Ok(con)
}

pub async fn init_mongodb() -> Result<mongodb::Database> {
    let username = env::var("MONGO_USERNAME").unwrap_or_else(|_| "root".into());
    let password = env::var("MONGO_PASSWORD").unwrap_or_else(|_| "example".into());
    let host = env::var("MONGO_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = env::var("MONGO_PORT").unwrap_or_else(|_| "27017".into());
    let database = env::var("MONGO_DB").unwrap_or_else(|_| "messenger".into());

    let connection_url = format!("mongodb://{}:{}@{}:{}", username, password, host, port);

    let mut mongo_client_options = mongodb::options::ClientOptions::parse(connection_url).await?;

    mongo_client_options.connect_timeout = Some(Duration::from_secs(5));
    mongo_client_options.server_selection_timeout = Some(Duration::from_secs(2));

    let client = mongodb::Client::with_options(mongo_client_options)?;

    Ok(client.database(&*database))
}

pub async fn init_rabbitmq() -> Result<Arc<Mutex<lapin::Connection>>> {
    let addr = env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());

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
