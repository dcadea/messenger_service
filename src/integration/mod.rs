use std::fs::File;
use std::str::FromStr;
use std::time::Duration;

use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};

pub mod cache;
pub mod db;
pub mod idp;
pub mod pubsub;

#[derive(Clone)]
pub enum Environment {
    Local,
    Docker,
    Production,
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
                "docker" => Environment::Docker,
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
