use std::time::Duration;

use mongodb::{Client, Database};
use mongodb::options::ClientOptions;

pub async fn init_mongodb() -> Database {
    let username = std::env::var("MONGO_USERNAME").unwrap_or_else(|_| "root".into());
    let password = std::env::var("MONGO_PASSWORD").unwrap_or_else(|_| "example".into());
    let host = std::env::var("MONGO_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("MONGO_PORT").unwrap_or_else(|_| "27017".into());
    let database = std::env::var("MONGO_DB").unwrap_or_else(|_| "messenger".into());

    let connection_url = format!("mongodb://{}:{}@{}:{}/{}", username, password, host, port, database);

    let mut mongo_client_options = ClientOptions::parse(connection_url).await.unwrap();
    mongo_client_options.connect_timeout = Some(Duration::from_secs(5));
    mongo_client_options.server_selection_timeout = Some(Duration::from_secs(2));
    let client = Client::with_options(mongo_client_options).unwrap();
    let database = client.database(&*database);
    database
}