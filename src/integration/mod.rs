use std::env;
use std::str::FromStr;
use std::time::Duration;
use std::{fs::File, net::SocketAddr};

use axum::http::HeaderValue;
use axum_server::tls_openssl::OpenSSLConfig;
use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin};

pub mod cache;
pub mod db;
pub mod idp;
pub mod pubsub;
pub mod storage;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub enum Env {
    Local,
    Dev,
    Stage,
    Production,
}

impl Env {
    pub fn addr(&self) -> SocketAddr {
        match self {
            Self::Local => SocketAddr::from(([127, 0, 0, 1], 8000)),
            Self::Dev | Self::Stage => SocketAddr::from(([0, 0, 0, 0], 8000)),
            Self::Production => SocketAddr::from(([0, 0, 0, 0], 8443)),
        }
    }

    pub fn ssl_config(&self) -> Option<OpenSSLConfig> {
        match self {
            Self::Local | Self::Dev | Self::Stage => None,
            Self::Production => {
                let ssl_config = OpenSSLConfig::from_pem_file(
                    env::var("SSL_CERT_FILE").expect("SSL_CERT_FILE must be set"),
                    env::var("SSL_KEY_FILE").expect("SSL_KEY_FILE must be set"),
                )
                .expect("cert should be present and have read permission");
                Some(ssl_config)
            }
        }
    }

    pub fn allow_origin(&self) -> AllowOrigin {
        match self {
            Self::Local | Self::Dev => AllowOrigin::any(),
            Self::Stage | Self::Production => {
                let origins = env::var("ALLOW_ORIGIN")
                    .expect("ALLOW_ORIGIN must be set")
                    .split(',')
                    .map(HeaderValue::from_str)
                    .map(|r| r.expect("invalid ALLOW_ORIGIN value"))
                    .collect::<Vec<HeaderValue>>();
                AllowOrigin::list(origins)
            }
        }
    }

    pub fn allow_methods(&self) -> AllowMethods {
        match self {
            Self::Local | Self::Dev | Self::Stage | Self::Production => AllowMethods::any(),
        }
    }

    pub fn allow_headers(&self) -> AllowHeaders {
        match self {
            Self::Local | Self::Dev | Self::Stage | Self::Production => AllowHeaders::any(),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    env: Env,

    redis: cache::Config,
    mongo: db::Config,
    pubsub: pubsub::Config,
    s3: storage::Config,

    idp: idp::Config,
}

impl Config {
    pub const fn new(
        env: Env,
        redis: cache::Config,
        mongo: db::Config,
        pubsub: pubsub::Config,
        s3: storage::Config,
        idp: idp::Config,
    ) -> Self {
        Self {
            env,
            redis,
            mongo,
            pubsub,
            s3,
            idp,
        }
    }

    pub const fn env(&self) -> &Env {
        &self.env
    }

    pub const fn redis(&self) -> &cache::Config {
        &self.redis
    }

    pub const fn mongo(&self) -> &db::Config {
        &self.mongo
    }

    pub const fn pubsub(&self) -> &pubsub::Config {
        &self.pubsub
    }

    pub const fn s3(&self) -> &storage::Config {
        &self.s3
    }

    pub const fn idp(&self) -> &idp::Config {
        &self.idp
    }
}

impl Default for Config {
    fn default() -> Self {
        dotenv().ok();

        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        let level = LevelFilter::from_str(&rust_log).unwrap_or(LevelFilter::Info);
        let log_file = env::var("SERVICE_NAME")
            .map_or_else(|_| "service.log".into(), |pkg| format!("{pkg}.log"));

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
                File::create(log_file).expect("Failed to create log file"),
            ),
        ])
        .expect("Failed to initialize logger");

        let env = env::var("ENV")
            .map(|env| match env.as_str() {
                "local" => Env::Local,
                "dev" => Env::Dev,
                "stg" => Env::Stage,
                "prod" => Env::Production,
                _ => panic!("Invalid environment: {env}"),
            })
            .unwrap_or(Env::Local);

        let idp_cfg = idp::Config::new(
            env::var("CLIENT_ID").expect("CLIENT_ID must be set"),
            env::var("CLIENT_SECRET").expect("CLIENT_SECRET must be set"),
            env::var("REDIRECT_URL").expect("REDIRECT_URL must be set"),
            env::var("ISSUER").expect("ISSUER must be set"),
            env::var("AUDIENCE").expect("AUDIENCE must be set"),
            env::var("REQUIRED_CLAIMS")
                .expect("REQUIRED_CLAIMS must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>()
                .as_slice(),
            Duration::from_secs(
                env::var("TOKEN_TTL")
                    .unwrap_or_else(|_| "3600".into())
                    .parse()
                    .expect("Failed to parse TOKEN_TTL"),
            ),
        );

        Self {
            env,
            redis: cache::Config::env().unwrap_or_default(),
            mongo: db::Config::env().unwrap_or_default(),
            pubsub: pubsub::Config::env().unwrap_or_default(),
            s3: storage::Config::env().unwrap_or_default(),
            idp: idp_cfg,
        }
    }
}

pub fn init_http_client() -> reqwest::Client {
    match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            panic!("Failed to initialize HTTP client: {e}")
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Identicon(#[from] identicon_rs::error::IdenticonError),
    #[error(transparent)]
    Minio(#[from] minio::s3::error::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
