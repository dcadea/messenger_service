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

    let db = mongodb::Client::with_options(options).map(|client| client.database(&config.db))?;

    Ok(db)
}

#[cfg(test)]
pub mod tests {
    use testcontainers_modules::mongo::Mongo;
    use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::testcontainers::runners::AsyncRunner;
    use crate::integration::mongo::Config;

    pub struct TestContainer {
        mongo: ContainerAsync<Mongo>,
        pub config: Config,
    }

    impl TestContainer {
        pub async fn init() -> Self {
            let mongo = Mongo::default()
                .with_container_name("awg_test_mongo")
                .start()
                .await
                .unwrap();

            let config = Config {
                host: mongo.get_host().await.unwrap().to_string(),
                port: mongo.get_host_port_ipv4(27017).await.unwrap(),
                db: String::from("test_messenger"),
            };

            Self { mongo, config }
        }
    }
}
