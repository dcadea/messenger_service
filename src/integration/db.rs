pub mod mongo {
    use std::env;
    use std::time::Duration;

    use log::warn;

    use crate::talk;

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
        pub fn env() -> Option<Self> {
            let host = env::var("MONGO_HOST").ok();
            let port = env::var("MONGO_PORT")
                .unwrap_or_else(|_| "27017".to_string())
                .parse()
                .ok();
            let db = env::var("MONGO_DB").unwrap_or_else(|_e| String::from("messenger"));

            if let (Some(host), Some(port)) = (host, port) {
                Some(Self { host, port, db })
            } else {
                warn!("MONGO env is not configured");
                None
            }
        }

        pub fn connect(&self) -> mongodb::Database {
            let options = mongodb::options::ClientOptions::builder()
                .hosts(vec![mongodb::options::ServerAddress::Tcp {
                    host: self.host.clone(),
                    port: Some(self.port),
                }])
                .server_selection_timeout(Some(Duration::from_secs(2)))
                .connect_timeout(Some(Duration::from_secs(5)))
                .build();

            match mongodb::Client::with_options(options).map(|client| client.database(&self.db)) {
                Ok(db) => db,
                Err(e) => panic!("Failed to connect to MongoDB: {e}"),
            }
        }
    }

    #[cfg(test)]
    use testcontainers_modules::mongo::Mongo;

    #[cfg(test)]
    impl Config {
        pub async fn test(
            node: &testcontainers_modules::testcontainers::ContainerAsync<Mongo>,
        ) -> Self {
            let host = node.get_host().await.unwrap();
            let port = node.get_host_port_ipv4(27017).await.unwrap();
            Self {
                host: host.to_string(),
                port,
                db: "test".into(),
            }
        }
    }

    impl From<talk::Id> for mongodb::bson::Bson {
        fn from(_val: talk::Id) -> Self {
            // match oid::ObjectId::from_str(&val.0) {
            //     Ok(oid) => Self::ObjectId(oid),
            //     Err(_) => Self::String(val.0.clone()),
            // }
            todo!()
        }
    }
}

pub mod pg {
    use std::env;

    use diesel::{PgConnection, r2d2::ConnectionManager};
    use log::warn;

    #[derive(Clone)]
    pub struct Config {
        host: String,
        port: u16,
        db: String,
        credentials: Credentials,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                host: String::from("127.0.0.1"),
                port: 5432,
                db: String::from("messenger"),
                credentials: Credentials::default(),
            }
        }
    }

    impl Config {
        pub fn env() -> Option<Self> {
            let host = env::var("POSTGRES_HOST").ok();
            let port = env::var("POSTGRES_HOST")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .ok();
            let db = env::var("POSTGRES_DB").unwrap_or_else(|_e| String::from("messenger"));
            let credentials = env::var("POSTGRES_USER")
                .and_then(|u| env::var("POSTGRES_PASSWORD").map(|p| (u, p)))
                .map(|(user, password)| Credentials { user, password })
                .unwrap_or_default();

            if let (Some(host), Some(port)) = (host, port) {
                Some(Self {
                    host,
                    port,
                    db,
                    credentials,
                })
            } else {
                warn!("POSTGRES env is not configured");
                None
            }
        }

        pub fn connect(&self) -> r2d2::Pool<ConnectionManager<PgConnection>> {
            let database_url = format!(
                "postgres://{}:{}@{}:{}/{}",
                self.credentials.user, self.credentials.password, self.host, self.port, self.db
            );

            let manager = ConnectionManager::<PgConnection>::new(database_url);

            match r2d2::Pool::builder().build(manager) {
                Ok(pool) => pool,
                Err(e) => panic!("Failed to connect to PostgreSQL: {e}"),
            }
        }
    }

    #[derive(Clone)]
    struct Credentials {
        user: String,
        password: String,
    }

    impl Default for Credentials {
        fn default() -> Self {
            Self {
                user: String::from("postgres"),
                password: String::from("postgres"),
            }
        }
    }
}
