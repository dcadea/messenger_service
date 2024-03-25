use tokio::sync::mpsc::UnboundedSender;
use warp::Error;
use warp::ws::Message;

#[derive(Clone)]
pub struct Client {
    user_id: usize,
    topics: Vec<String>,
    sender: Option<UnboundedSender<Result<Message, Error>>>,
}

impl Client {
    pub fn new(user_id: usize, topics: Vec<String>, sender: Option<UnboundedSender<Result<Message, Error>>>) -> Self {
        Self {
            user_id,
            topics,
            sender,
        }
    }

    pub fn user_id(&self) -> usize {
        self.user_id
    }

    pub fn topics(&self) -> Vec<String> {
        self.topics.clone()
    }

    pub fn set_topics(&mut self, topics: Vec<String>) {
        self.topics = topics;
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
    user_id: Option<usize>,
    message: String,
}

impl Event {
    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }

    pub fn user_id(&self) -> Option<usize> {
        self.user_id
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct RegisterRequest {
    user_id: usize,
}

impl RegisterRequest {
    pub fn user_id(&self) -> usize {
        self.user_id
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


