use std::time::Duration;

use mongodb::{Client, Database};
use mongodb::options::ClientOptions;

const MONGO_URI: &str = "mongodb://root:example@localhost:27017";
const MONGO_DB: &str = "messenger";

pub async fn init_mongodb() -> Database {
    let mut mongo_client_options = ClientOptions::parse(MONGO_URI).await.unwrap();
    mongo_client_options.connect_timeout = Some(Duration::from_secs(5));
    mongo_client_options.server_selection_timeout = Some(Duration::from_secs(2));
    let client = Client::with_options(mongo_client_options).unwrap();
    let database = client.database(MONGO_DB);
    database
}