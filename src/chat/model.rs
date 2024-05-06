use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct Chat {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    nickname: String,
    recipient: String,
    last_message: String,
}

impl Chat {
    pub fn from_request(nickname: &str, chat_request: ChatRequest) -> Self {
        Self {
            _id: None,
            nickname: nickname.to_string(),
            recipient: chat_request.recipient,
            last_message: chat_request.last_message,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct ChatRequest {
    recipient: String,
    last_message: String,
}
