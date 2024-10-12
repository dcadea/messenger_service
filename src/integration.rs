use std::fs::File;
use std::str::FromStr;
use std::time::Duration;

use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub(crate) struct Config {
    pub redis: cache::Config,
    pub mongo: db::Config,
    pub amqp: amqp::Config,

    pub idp: idp::Config,
}

impl Default for Config {
    fn default() -> Self {
        dotenv().ok();

        let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into());
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

        let idp_config = idp::Config::new(
            std::env::var("CLIENT_ID").expect("CLIENT_ID must be set"),
            std::env::var("CLIENT_SECRET").expect("CLIENT_SECRET must be set"),
            std::env::var("REDIRECT_URL").expect("REDIRECT_URL must be set"),
            std::env::var("ISSUER").expect("ISSUER must be set"),
            std::env::var("AUDIENCE").expect("AUDIENCE must be set"),
            std::env::var("REQUIRED_CLAIMS")
                .expect("REQUIRED_CLAIMS must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>(),
        );

        Self {
            redis: cache::Config::env().unwrap_or_default(),
            mongo: db::Config::env().unwrap_or_default(),
            amqp: amqp::Config::env().unwrap_or_default(),
            idp: idp_config,
        }
    }
}

pub(crate) mod amqp {
    use std::env;

    use lapin::uri::{AMQPAuthority, AMQPQueryString, AMQPScheme, AMQPUri, AMQPUserInfo};
    use tokio::sync::RwLock;

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
        pub fn env() -> super::Result<Self> {
            let host = env::var("AMQP_HOST")?;
            let port = env::var("AMQP_PORT")?.parse()?;
            Ok(Self { host, port })
        }
    }

    pub async fn init(config: &Config) -> super::Result<RwLock<lapin::Connection>> {
        let amqp_uri = AMQPUri {
            scheme: AMQPScheme::AMQP,
            authority: AMQPAuthority {
                userinfo: AMQPUserInfo::default(),
                host: config.host.to_owned(),
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

pub(crate) mod cache {
    use std::env;
    use std::fmt::{Display, Formatter};

    use crate::user::model::UserInfo;
    use crate::{chat, user};

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
        pub fn env() -> super::Result<Self> {
            let host = env::var("REDIS_HOST")?;
            let port = env::var("REDIS_PORT")?.parse()?;
            Ok(Self { host, port })
        }
    }

    pub async fn init_client(config: &Config) -> super::Result<redis::Client> {
        redis::Client::open(format!("redis://{}:{}", &config.host, &config.port))
            .map_err(super::Error::from)
    }

    pub async fn init(config: &Config) -> super::Result<redis::aio::ConnectionManager> {
        init_client(config)
            .await?
            .get_connection_manager()
            .await
            .map_err(super::Error::from)
    }

    #[derive(Clone)]
    pub enum Key {
        UserInfo(user::Sub),
        UsersOnline,
        Friends(user::Sub),
        Chat(chat::Id),
    }

    impl Display for Key {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Key::UserInfo(sub) => write!(f, "userinfo:{sub}"),
                Key::UsersOnline => write!(f, "users:online"),
                Key::Friends(sub) => write!(f, "friends:{sub}"),
                Key::Chat(id) => write!(f, "chat:{id}"),
            }
        }
    }

    impl redis::ToRedisArgs for Key {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            self.to_string().write_redis_args(out);
        }
    }

    #[derive(Clone)]
    pub struct Keyspace {
        pub key: Key,
    }

    impl Keyspace {
        pub fn new(key: Key) -> Self {
            Self { key }
        }
    }

    impl Display for Keyspace {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "__keyspace@0__:{}", &self.key)
        }
    }

    impl redis::ToRedisArgs for Keyspace {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            self.to_string().write_redis_args(out);
        }
    }

    impl redis::FromRedisValue for user::Sub {
        fn from_redis_value(v: &redis::Value) -> redis::RedisResult<user::Sub> {
            let s = String::from_redis_value(v)?;
            Ok(user::Sub(s))
        }
    }

    impl redis::ToRedisArgs for user::Sub {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            self.0.write_redis_args(out);
        }
    }

    impl redis::FromRedisValue for UserInfo {
        fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
            let user_info: UserInfo = serde_json::from_str(&String::from_redis_value(value)?)?;
            Ok(user_info)
        }
    }

    impl redis::ToRedisArgs for UserInfo {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            serde_json::json!(self).to_string().write_redis_args(out);
        }
    }
}

pub(crate) mod db {
    use std::env;
    use std::time::Duration;

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
        pub fn env() -> super::Result<Self> {
            let host = env::var("MONGO_HOST")?;
            let port = env::var("MONGO_PORT")?.parse()?;
            let db = env::var("MONGO_DB")?;
            Ok(Self { host, port, db })
        }
    }

    pub async fn init(config: &Config) -> super::Result<mongodb::Database> {
        let options = mongodb::options::ClientOptions::builder()
            .hosts(vec![mongodb::options::ServerAddress::Tcp {
                host: config.host.to_owned(),
                port: Some(config.port),
            }])
            .server_selection_timeout(Some(Duration::from_secs(2)))
            .connect_timeout(Some(Duration::from_secs(5)))
            .build();

        let db =
            mongodb::Client::with_options(options).map(|client| client.database(&config.db))?;

        Ok(db)
    }

    impl From<crate::user::Sub> for mongodb::bson::Bson {
        fn from(val: crate::user::Sub) -> Self {
            mongodb::bson::Bson::String(val.0)
        }
    }
}

pub(crate) mod idp {
    use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

    #[derive(Clone)]
    pub struct Config {
        pub client_id: String,
        pub client_secret: String,
        pub auth_url: String,
        pub token_url: String,
        pub redirect_url: String,
        pub userinfo_url: String,
        pub jwks_url: String,
        pub issuer: String,
        pub audience: String,
        pub required_claims: Vec<String>,
    }

    impl Config {
        pub fn new(
            client_id: String,
            client_secret: String,
            redirect_url: String,
            issuer: String,
            audience: String,
            required_claims: Vec<String>,
        ) -> Self {
            Self {
                client_id,
                client_secret,
                auth_url: format!("{issuer}authorize"),
                token_url: format!("{issuer}oauth/token"),
                redirect_url,
                userinfo_url: format!("{issuer}userinfo"),
                jwks_url: format!("{issuer}.well-known/jwks.json"),
                issuer,
                audience,
                required_claims,
            }
        }
    }

    pub fn init(config: &Config) -> BasicClient {
        let client_id = ClientId::new(config.client_id.to_owned());
        let client_secret = ClientSecret::new(config.client_secret.to_owned());
        let auth_url = AuthUrl::new(config.auth_url.to_owned()).expect("Invalid authorization URL");
        let token_url = TokenUrl::new(config.token_url.to_owned()).expect("Invalid token URL");
        let redirect_url =
            RedirectUrl::new(config.redirect_url.to_owned()).expect("Invalid redirect URL");

        BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url)
    }
}

pub(crate) fn init_http_client() -> self::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(Error::from)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    _Var(#[from] std::env::VarError),
    _ParseInt(#[from] std::num::ParseIntError),
    _MongoDB(#[from] mongodb::error::Error),
    _Lapin(#[from] lapin::Error),
    _Redis(#[from] redis::RedisError),
    _Reqwest(#[from] reqwest::Error),
}
