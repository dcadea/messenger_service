use serde::{Deserialize, Serialize};

use crate::user;

use super::{Id, Status, StatusTransition};

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    pub sub1: user::Sub,
    pub sub2: user::Sub,
    pub status: Status,
}

impl Contact {
    pub fn new(sub1: user::Sub, sub2: user::Sub) -> Self {
        Self {
            id: None,
            sub1: sub1.clone(),
            sub2,
            status: Status::Pending { initiator: sub1 },
        }
    }

    pub fn transition(&mut self, t: StatusTransition) -> bool {
        let mut changed = true;

        match (&self.status, t) {
            (Status::Pending { initiator }, StatusTransition::Accept { responder }) => {
                if initiator.eq(&responder) {
                    return false;
                }
                self.status = Status::Accepted;
            }
            (Status::Pending { initiator }, StatusTransition::Reject { responder }) => {
                if initiator.eq(&responder) {
                    return false;
                }
                self.status = Status::Rejected;
            }
            (Status::Accepted, StatusTransition::Block { initiator }) => {
                self.status = Status::Blocked { initiator }
            }
            (Status::Blocked { initiator }, StatusTransition::Unblock { target }) => {
                if initiator.eq(&target) {
                    return false;
                }
                self.status = Status::Accepted;
            }
            (_, _) => {
                changed = false; /* no change */
            }
        };

        changed
    }
}

pub struct ContactDto {
    pub id: Id,
    pub recipient: user::Sub,
    pub status: Status,
}
