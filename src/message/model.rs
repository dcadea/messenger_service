use bson::oid::ObjectId;
use mongodb::bson;
use serde::{Deserialize, Serialize, Serializer};

pub type MessageId = ObjectId;

#[derive(Deserialize, Serialize, Clone)]
pub struct Message {
    #[serde(
        alias = "_id",
        serialize_with = "serialize_message_id",
        skip_serializing_if = "Option::is_none"
    )]
    id: Option<MessageId>,
    pub sender: String,
    recipient: String,
    text: String,
    timestamp: i64,
    seen: bool,
}

impl Message {
    pub fn new(sender: &str, recipient: &str, text: &str) -> Self {
        Self {
            id: None,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            text: text.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            seen: false,
        }
    }
}

#[derive(Deserialize)]
pub(super) struct MessageParams {
    pub recipient: Option<Vec<String>>,
}

fn serialize_message_id<S>(message_id: &Option<MessageId>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match message_id {
        Some(ref message_id) => serializer.serialize_some(message_id.to_hex().as_str()),
        None => serializer.serialize_none(),
    }
}
