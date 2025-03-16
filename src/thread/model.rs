use serde::{Deserialize, Serialize};

use crate::{message::model::LastMessage, user};

use super::Id;

#[derive(Clone, Serialize, Deserialize)]
pub struct Thread {
    _id: Id,
    details: Details,
    last_message: Option<LastMessage>,
}

#[derive(Clone, Serialize, Deserialize)]
enum Details {
    Chat {
        members: [user::Sub; 2],
    },
    Group {
        owner: user::Sub,
        members: Vec<user::Sub>,
    },
}
