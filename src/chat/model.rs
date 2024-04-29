use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Chat {
    _id: Option<bson::oid::ObjectId>,
    username: String,
    recipient: String,
    last_message: String,
}
