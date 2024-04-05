use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use warp::ws::Message;
use warp::Error;

#[derive(Serialize, Deserialize, Clone)]
pub struct WsClient {
    username: String,
    topics: Vec<String>,
    #[serde(skip)]
    sender: Option<UnboundedSender<Result<Message, Error>>>,
}

impl WsClient {
    pub fn new(
        username: String,
        topics: Vec<String>,
        sender: Option<UnboundedSender<Result<Message, Error>>>,
    ) -> Self {
        Self {
            username,
            topics,
            sender,
        }
    }

    pub fn username(&self) -> String {
        self.username.clone()
    }

    pub fn topics(&self) -> Vec<String> {
        self.topics.clone()
    }

    pub fn sender(&self) -> Option<UnboundedSender<Result<Message, Error>>> {
        self.sender.clone()
    }

    pub fn set_sender(&mut self, sender: UnboundedSender<Result<Message, Error>>) {
        self.sender = Some(sender);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Event {
    topic: String,
    username: Option<String>,
    message: String,
}

impl Event {
    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }

    pub fn username(&self) -> Option<String> {
        self.username.clone()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct RegisterResponse {
    url: String,
}

impl RegisterResponse {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct TopicsRequest {
    topics: Vec<String>,
}

impl TopicsRequest {
    pub fn topics(&self) -> Vec<String> {
        self.topics.clone()
    }
}
