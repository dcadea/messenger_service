use std::fs::File;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use dotenv::dotenv;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};

pub mod amqp;
pub mod cache;
pub mod db;
pub mod idp;
pub mod model;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    _Var(#[from] std::env::VarError),
    _ParseInt(#[from] std::num::ParseIntError),
    _MongoDB(#[from] mongodb::error::Error),
    _Lapin(#[from] lapin::Error),
    _Redis(#[from] redis::RedisError),
    _Reqwest(#[from] reqwest::Error),
}

#[derive(Clone)]
pub struct Config {
    pub socket: SocketAddr,

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

        let app_addr = std::env::var("APP_ADDR").unwrap_or("127.0.0.1".into());
        let app_port = std::env::var("APP_PORT").unwrap_or("8000".into());

        let socket = format!("{}:{}", app_addr, app_port)
            .parse()
            .expect("Failed to parse socket address");

        let idp_config = idp::Config::new(
            std::env::var("ISSUER").expect("ISSUER must be set"),
            std::env::var("AUDIENCE")
                .expect("AUDIENCE must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>(),
            std::env::var("REQUIRED_CLAIMS")
                .expect("REQUIRED_CLAIMS must be set")
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>(),
        );

        Self {
            socket,
            redis: cache::Config::env().unwrap_or_default(),
            mongo: db::Config::env().unwrap_or_default(),
            amqp: amqp::Config::env().unwrap_or_default(),
            idp: idp_config,
        }
    }
}

pub fn init_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(Error::from)
}
