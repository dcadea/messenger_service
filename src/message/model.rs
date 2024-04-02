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
