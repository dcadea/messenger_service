use mongodb::bson;
use tokio::sync::mpsc;
use warp::ws::Message;

#[derive(Clone)]
pub struct Client {
    pub user_id: usize,
    pub topics: Vec<String>,
    pub sender: Option<mpsc::UnboundedSender<Result<Message, warp::Error>>>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Event {
    topic: String,
    user_id: Option<usize>,
    message: String,
}

impl Event {
    pub fn topic(&self) -> String {
        self.topic.clone()
    }

    pub fn user_id(&self) -> Option<usize> {
        self.user_id
    }

    pub fn message(&self) -> String {
        self.message.clone()
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

#[derive(serde::Deserialize, serde::Serialize)]
pub struct User {
    #[serde(skip)]
    pub _id: Option<bson::oid::ObjectId>,
    pub username: String,
    pub password: String,
}

impl User {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            _id: None,
            username: String::from(username),
            password: String::from(password),
        }
    }

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }
}

#[derive(serde::Serialize)]
pub struct UserResponse {
    pub username: String,
}

impl UserResponse {
    pub fn new(username: &str) -> Self {
        Self { username: String::from(username) }
    }
}