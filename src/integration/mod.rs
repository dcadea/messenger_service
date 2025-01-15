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

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub enum Environment {
    Local,
    Dev,
    Stage,
    Production,
}

impl Environment {
    pub fn addr(&self) -> SocketAddr {
        match self {
            Environment::Local => SocketAddr::from(([127, 0, 0, 1], 8000)),
            Environment::Dev | Environment::Stage => SocketAddr::from(([0, 0, 0, 0], 8000)),
            Environment::Production => SocketAddr::from(([0, 0, 0, 0], 8443)),
        }
    }

    pub fn ssl_config(&self) -> Option<OpenSSLConfig> {
        match self {
            Environment::Local | Environment::Dev | Environment::Stage => None,
            Environment::Production => {
                let ssl_config = OpenSSLConfig::from_pem_file(
                    std::env::var("SSL_CERT_FILE").expect("SSL_CERT_FILE must be set"),
                    std::env::var("SSL_KEY_FILE").expect("SSL_KEY_FILE must be set"),
                )
                .expect("cert should be present and have read permission");
                Some(ssl_config)
            }
        }
    }

    pub fn allow_origin(&self) -> AllowOrigin {
        match self {
            Environment::Local | Environment::Dev => AllowOrigin::any(),
            Environment::Stage | Environment::Production => {
                let origins = std::env::var("ALLOW_ORIGIN")
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
            Environment::Local | Environment::Dev => AllowMethods::any(),
            Environment::Stage | Environment::Production => AllowMethods::any(),
        }
    }

    pub fn allow_headers(&self) -> AllowHeaders {
        match self {
            Environment::Local | Environment::Dev => AllowHeaders::any(),
            Environment::Stage | Environment::Production => AllowHeaders::any(),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub env: Environment,

    pub redis: cache::Config,
    pub mongo: db::Config,
    pub pubsub: pubsub::Config,

    pub idp: idp::Config,
}

impl Default for Config {
    fn default() -> Self {
        dotenv().ok();

        let rust_log = std::env::var("RUST_LOG").unwrap_or("info".into());
        let level = LevelFilter::from_str(&rust_log).unwrap_or(LevelFilter::Info);
        let log_file = std::env::var("SERVICE_NAME")
            .map(|pkg| format!("{}.log", pkg))
            .unwrap_or("service.log".into());

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

        let env = std::env::var("ENV")
            .map(|env| match env.as_str() {
                "local" => Environment::Local,
                "dev" => Environment::Dev,
                "stg" => Environment::Stage,
                "prod" => Environment::Production,
                _ => panic!("Invalid environment: {}", env),
            })
            .unwrap_or(Environment::Local);

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
            Duration::from_secs(
                std::env::var("TOKEN_TTL")
                    .unwrap_or("3600".into())
                    .parse()
                    .expect("Failed to parse TOKEN_TTL"),
            ),
        );

        Self {
            env,
            redis: cache::Config::env().unwrap_or_default(),
            mongo: db::Config::env().unwrap_or_default(),
            pubsub: pubsub::Config::env().unwrap_or_default(),
            idp: idp_config,
        }
    }
}

pub fn init_http_client() -> reqwest::Client {
    match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            panic!("Failed to initialize HTTP client: {}", e)
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    _Var(#[from] std::env::VarError),

    #[error(transparent)]
    _ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    _Redis(#[from] redis::RedisError),
}
