use serde::{Deserialize, Serialize};

pub type ChatId = mongodb::bson::oid::ObjectId;

#[derive(Serialize, Deserialize)]
pub struct Chat {
    #[serde(skip)]
    _id: Option<ChatId>,
    sender: String,
    recipient: String,
    last_message: String,
}

impl Chat {
    pub fn from_request(sender: &str, chat_request: ChatRequest) -> Self {
        Self {
            _id: None,
            sender: sender.to_string(),
            recipient: chat_request.recipient,
            last_message: chat_request.last_message,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    recipient: String,
    last_message: String,
}
