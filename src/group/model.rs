use serde::{Deserialize, Serialize};

use crate::user;

use super::Id;

#[derive(Serialize, Deserialize)]
struct Group {
    #[serde(alias = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    name: String,
    owner: user::Sub,
    participants: Vec<user::Sub>,
    picture: String,
    last_message: String,
}
