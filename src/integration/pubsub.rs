use std::{env, fmt};

use bytes::Bytes;
use log::{error, warn};

use crate::event;

#[derive(Clone)]
pub struct Config {
    host: String,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 4222,
        }
    }
}

impl Config {
    pub fn env() -> Option<Self> {
        let host = env::var("NATS_HOST").ok();
        let port = env::var("NATS_PORT")
            .unwrap_or_else(|_| "4222".to_string())
            .parse()
            .ok();

        if let (Some(host), Some(port)) = (host, port) {
            Some(Self { host, port })
        } else {
            warn!("NATS env is not configured");
            None
        }
    }

    pub async fn connect(&self) -> async_nats::Client {
        match async_nats::connect(&format!("{}:{}", self.host, self.port)).await {
            Ok(con) => con,
            Err(e) => panic!("Failed to connect to NATS: {e}"),
        }
    }
}

impl async_nats::subject::ToSubject for &event::Subject<'_> {
    fn to_subject(&self) -> async_nats::Subject {
        match self {
            event::Subject::Notifications(sub) => format!("noti.{sub}").into(),
            event::Subject::Messages(sub, talk_id) => format!("messages.{sub}.{talk_id}").into(),
        }
    }
}

impl fmt::Display for &event::Subject<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            event::Subject::Notifications(sub) => write!(f, "noti.{sub}"),
            event::Subject::Messages(sub, talk_id) => write!(f, "messages.{sub}.{talk_id}"),
        }
    }
}

impl From<event::Notification> for Bytes {
    fn from(n: event::Notification) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        if let Err(e) = serde_json::to_writer(&mut bytes, &n) {
            error!("could not serialize notification: {e:?}");
        }
        bytes.into()
    }
}

impl From<event::Message> for Bytes {
    fn from(m: event::Message) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        if let Err(e) = serde_json::to_writer(&mut bytes, &m) {
            error!("could not serialize event message: {e:?}");
        }
        bytes.into()
    }
}
