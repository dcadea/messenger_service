use std::env;

use lapin::uri::{AMQPAuthority, AMQPQueryString, AMQPScheme, AMQPUri, AMQPUserInfo};
use tokio::sync::RwLock;

use crate::integration::Result;

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
    pub fn env() -> Result<Self> {
        let host = env::var("AMQP_HOST")?;
        let port = env::var("AMQP_PORT")?.parse()?;
        Ok(Self { host, port })
    }
}

pub async fn init(config: &Config) -> Result<RwLock<lapin::Connection>> {
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
