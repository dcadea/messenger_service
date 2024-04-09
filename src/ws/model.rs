#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Event {
    sender: String,
    recipient: String,
    message: String,
}

impl Event {
    pub fn sender(&self) -> &str {
        self.sender.as_str()
    }

    pub fn recipient(&self) -> &str {
        self.recipient.as_str()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}
