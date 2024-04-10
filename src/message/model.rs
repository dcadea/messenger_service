use chrono::Utc;
use mongodb::bson;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Message {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    sender: String,
    recipient: String,
    text: String,
    timestamp: i64,
    seen: bool,
}

impl Message {
    pub fn new(sender: &str, recipient: &str, text: &str, timestamp: i64) -> Self {
        Self {
            _id: None,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            text: text.to_string(),
            timestamp,
            seen: false,
        }
    }

    pub fn sender(&self) -> &str {
        &self.sender
    }

    pub fn recipient(&self) -> &str {
        &self.recipient
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct MessageRequest {
    sender: String,
    recipient: String,
    text: String,
}

impl MessageRequest {
    pub fn sender(&self) -> &str {
        self.sender.as_str()
    }

    pub fn recipient(&self) -> &str {
        self.recipient.as_str()
    }

    pub fn text(&self) -> &str {
        self.text.as_str()
    }
}

impl From<MessageRequest> for Message {
    fn from(request: MessageRequest) -> Self {
        Self {
            _id: None,
            sender: request.sender().to_string(),
            recipient: request.recipient().to_string(),
            text: request.text().to_string(),
            timestamp: Utc::now().timestamp(),
            seen: false,
        }
    }
}
