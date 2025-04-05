use serde::{Deserialize, Serialize};

use crate::user;

use super::{Id, Status};

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    pub sub1: user::Sub,
    pub sub2: user::Sub,
    pub status: Status,
}

impl Contact {
    pub fn new(sub1: user::Sub, sub2: user::Sub) -> Self {
        Self {
            id: None,
            sub1,
            sub2,
            status: Status::Pending,
        }
    }
}

impl From<[user::Sub; 2]> for Contact {
    fn from(v: [user::Sub; 2]) -> Self {
        Self::new(v[0].clone(), v[1].clone())
    }
}

pub struct ContactDto {
    pub recipient: user::Sub,
    pub status: Status,
}
