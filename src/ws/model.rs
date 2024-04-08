#[derive(serde::Deserialize, serde::Serialize)]
pub struct Event {
    topic: String,
    message: String,
}

impl Event {
    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}
