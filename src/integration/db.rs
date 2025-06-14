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
