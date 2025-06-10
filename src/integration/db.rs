pub mod mongo {
    use std::time::Duration;
    use std::{env, str::FromStr};

    use log::warn;
    use mongodb::bson::{doc, oid};

    use crate::{contact, message, talk, user};

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

    impl From<contact::Id> for mongodb::bson::Bson {
        fn from(val: contact::Id) -> Self {
            match oid::ObjectId::from_str(&val.0) {
                Ok(oid) => Self::ObjectId(oid),
                Err(_) => Self::String(val.0.clone()),
            }
        }
    }

    impl From<contact::Status> for mongodb::bson::Bson {
        fn from(val: contact::Status) -> Self {
            let doc = match val {
                contact::Status::Pending { initiator } => {
                    doc! { "indicator": "pending", "initiator": initiator }
                }
                contact::Status::Accepted => doc! {"indicator": "accepted"},
                contact::Status::Rejected => doc! {"indicator": "rejected"},
                contact::Status::Blocked { initiator } => {
                    doc! {"indicator": "blocked", "initiator": initiator}
                }
            };

            Self::Document(doc)
        }
    }

    impl From<talk::Id> for mongodb::bson::Bson {
        fn from(val: talk::Id) -> Self {
            // match oid::ObjectId::from_str(&val.0) {
            //     Ok(oid) => Self::ObjectId(oid),
            //     Err(_) => Self::String(val.0.clone()),
            // }
            todo!()
        }
    }

    impl From<message::Id> for mongodb::bson::Bson {
        fn from(id: message::Id) -> Self {
            // match oid::ObjectId::from_str(&id.0) {
            //     Ok(oid) => Self::ObjectId(oid),
            //     Err(_) => Self::String(id.0.clone()), // FIXME: implement TryFrom
            // }
            todo!()
        }
    }

    impl From<user::Id> for mongodb::bson::Bson {
        fn from(id: user::Id) -> Self {
            // match oid::ObjectId::from_str(id.as_str()) {
            //     Ok(oid) => Self::ObjectId(oid),
            //     Err(_) => Self::String(id.as_str().to_string()), // FIXME: implement TryFrom
            // }
            //
            todo!()
        }
    }

    impl From<user::Sub> for mongodb::bson::Bson {
        fn from(val: user::Sub) -> Self {
            Self::String(val.to_string())
        }
    }

    impl From<user::Nickname> for mongodb::bson::Bson {
        fn from(val: user::Nickname) -> Self {
            Self::String(val.as_str().to_string())
        }
    }

    impl From<message::model::LastMessage> for mongodb::bson::Bson {
        fn from(lm: message::model::LastMessage) -> Self {
            // Self::Document(doc! {
            //     "id": lm.id(),
            //     "text": lm.text(),
            //     "owner": lm.owner(),
            //     "timestamp": lm.timestamp(),
            //     "seen": lm.seen()
            // })
            todo!()
        }
    }
}

pub mod pg {
    use std::env;

    use diesel::{PgConnection, deserialize::FromSqlRow, r2d2::ConnectionManager};
    use log::warn;

    use crate::user;

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
