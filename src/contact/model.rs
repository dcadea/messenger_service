use diesel::prelude::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::user;

use super::{Id, Status, StatusTransition};

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::contacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Contact {
    id: Id,
    user_id_1: user::Id,
    user_id_2: user::Id,
    status: String,
    initiator: Option<user::Id>,
}

impl Contact {
    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub const fn user_id_1(&self) -> &user::Id {
        &self.user_id_1
    }

    pub const fn user_id_2(&self) -> &user::Id {
        &self.user_id_2
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub const fn initiator(&self) -> Option<&user::Id> {
        self.initiator.as_ref()
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::contacts)]
pub struct NewContact<'a> {
    user_id_1: &'a user::Id,
    user_id_2: &'a user::Id,
    status: &'a str,
    initiator: &'a user::Id,
}

impl<'a> NewContact<'a> {
    pub const fn new(initiator: &'a user::Id, responder: &'a user::Id) -> Self {
        Self {
            user_id_1: initiator,
            user_id_2: responder,
            status: "pending",
            initiator,
        }
    }

    pub const fn user_id_1(&self) -> &user::Id {
        self.user_id_1
    }

    pub const fn user_id_2(&self) -> &user::Id {
        self.user_id_2
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ContactDto {
    id: Id,
    sender: user::Id,
    recipient: user::Id,
    status: Status,
}

impl ContactDto {
    pub const fn new(id: Id, sender: user::Id, recipient: user::Id, status: Status) -> Self {
        Self {
            id,
            sender,
            recipient,
            status,
        }
    }

    pub const fn id(&self) -> &Id {
        &self.id
    }

    pub const fn recipient(&self) -> &user::Id {
        &self.recipient
    }

    pub const fn status(&self) -> &Status {
        &self.status
    }

    pub const fn is_accepted(&self) -> bool {
        matches!(self.status, Status::Accepted)
    }
}

impl ContactDto {
    /// Possible transitions:
    /// - Pending -> (Accept) -> Accepted
    /// - Pending -> (Reject) -> Rejected
    /// - Accepted -> (Block) -> Blocked
    /// - Blocked -> (Unblock) -> Accepted
    pub fn transition(&self, t: StatusTransition) -> super::Result<Status> {
        match (&self.status, t) {
            (Status::Pending { initiator }, StatusTransition::Accept { responder }) => {
                if !self.is_member(responder) {
                    return Err(super::Error::StatusTransitionFailed);
                }

                if initiator.eq(responder) {
                    return Err(super::Error::StatusTransitionFailed);
                }

                Ok(Status::Accepted)
            }
            (Status::Pending { initiator }, StatusTransition::Reject { responder }) => {
                if !self.is_member(responder) {
                    return Err(super::Error::StatusTransitionFailed);
                }

                if initiator.eq(responder) {
                    return Err(super::Error::StatusTransitionFailed);
                }

                Ok(Status::Rejected)
            }
            (Status::Accepted, StatusTransition::Block { initiator }) => {
                if !self.is_member(initiator) {
                    return Err(super::Error::StatusTransitionFailed);
                }

                Ok(Status::Blocked {
                    initiator: initiator.clone(),
                })
            }
            (Status::Blocked { initiator }, StatusTransition::Unblock { target }) => {
                if !self.is_member(target) {
                    return Err(super::Error::StatusTransitionFailed);
                }

                if initiator.eq(target) {
                    return Err(super::Error::StatusTransitionFailed);
                }

                Ok(Status::Accepted)
            }
            _ => Err(super::Error::StatusTransitionFailed), /* no change */
        }
    }

    fn is_member(&self, id: &user::Id) -> bool {
        if self.sender.eq(id) {
            return true;
        }

        if self.recipient.eq(id) {
            return true;
        }

        false
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Contacts {
    c: Vec<ContactDto>,
}

impl Contacts {
    pub fn from_ref(c: &[ContactDto]) -> Self {
        Self { c: c.to_owned() }
    }

    pub const fn get(&self) -> &Vec<ContactDto> {
        &self.c
    }
}
