use diesel::prelude::{Insertable, Queryable, Selectable};

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
    pub fn new(initiator: &'a user::Id, responder: &'a user::Id) -> Self {
        Self {
            user_id_1: initiator,
            user_id_2: responder,
            status: "pending",
            initiator,
        }
    }

    pub const fn user_id_1(&self) -> &user::Id {
        &self.user_id_1
    }

    pub const fn user_id_2(&self) -> &user::Id {
        &self.user_id_2
    }
}

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
    pub fn transition(&mut self, t: StatusTransition) -> bool {
        match (&self.status, t) {
            (Status::Pending { initiator }, StatusTransition::Accept { responder }) => {
                if !self.is_member(responder) {
                    return false;
                }

                if initiator.eq(responder) {
                    return false;
                }

                self.status = Status::Accepted;
                true
            }
            (Status::Pending { initiator }, StatusTransition::Reject { responder }) => {
                if !self.is_member(responder) {
                    return false;
                }

                if initiator.eq(responder) {
                    return false;
                }

                self.status = Status::Rejected;
                true
            }
            (Status::Accepted, StatusTransition::Block { initiator }) => {
                if !self.is_member(initiator) {
                    return false;
                }

                self.status = Status::Blocked {
                    initiator: initiator.clone(),
                };
                true
            }
            (Status::Blocked { initiator }, StatusTransition::Unblock { target }) => {
                if !self.is_member(target) {
                    return false;
                }

                if initiator.eq(target) {
                    return false;
                }

                self.status = Status::Accepted;
                true
            }
            _ => false, /* no change */
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

// #[cfg(test)]
// impl Contact {
//     pub fn set_status(&mut self, status: Status) {
//         self.status = status;
//     }
// }

// FIXME
// #[cfg(test)]
// mod test {
//     use crate::{
//         contact::{Status, StatusTransition},
//         user::Sub,
//     };

//     use super::Contact;

//     #[test]
//     fn should_create_in_pending_state() {
//         let initiator = Sub::from("123");
//         let c = Contact::new(&initiator, &Sub::from("456"));

//         assert_eq!(c.status, Status::Pending { initiator });
//     }

//     #[test]
//     fn should_transition_from_pending_to_accepted() {
//         let responder = &Sub::from("456");
//         let mut c = Contact::new(&Sub::from("123"), responder);

//         let transitioned = c.transition(StatusTransition::Accept { responder });

//         assert!(transitioned);
//         assert_eq!(c.status, Status::Accepted);
//     }

//     #[test]
//     fn should_not_transition_from_pending_to_accepted_when_responder_not_a_member() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &Sub::from("456"));

//         let transitioned = c.transition(StatusTransition::Accept {
//             responder: &Sub::from("789"),
//         });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Pending { initiator });
//     }

//     #[test]
//     fn should_not_transition_from_pending_to_accepted_when_same_subs() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &initiator);

//         let transitioned = c.transition(StatusTransition::Accept {
//             responder: &initiator,
//         });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Pending { initiator });
//     }

//     #[test]
//     fn should_transition_from_pending_to_rejected() {
//         let responder = &Sub::from("456");
//         let mut c = Contact::new(&Sub::from("123"), responder);

//         let transitioned = c.transition(StatusTransition::Reject { responder });

//         assert!(transitioned);
//         assert_eq!(c.status, Status::Rejected);
//     }

//     #[test]
//     fn should_not_transition_from_pending_to_rejected_when_responder_not_a_member() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &Sub::from("456"));

//         let transitioned = c.transition(StatusTransition::Reject {
//             responder: &Sub::from("789"),
//         });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Pending { initiator });
//     }

//     #[test]
//     fn should_not_transition_from_pending_to_rejected_when_same_subs() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &initiator);

//         let transitioned = c.transition(StatusTransition::Reject {
//             responder: &initiator,
//         });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Pending { initiator });
//     }

//     #[test]
//     fn should_transition_from_accepted_to_blocked_when_initiator_is_sub1() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &Sub::from("456"));
//         c.set_status(Status::Accepted);

//         let transitioned = c.transition(StatusTransition::Block {
//             initiator: &initiator,
//         });

//         assert!(transitioned);
//         assert_eq!(c.status, Status::Blocked { initiator });
//     }

//     #[test]
//     fn should_transition_from_accepted_to_blocked_when_initiator_is_sub2() {
//         let initiator = Sub::from("456");
//         let mut c = Contact::new(&Sub::from("123"), &initiator);
//         c.set_status(Status::Accepted);

//         let transitioned = c.transition(StatusTransition::Block {
//             initiator: &initiator,
//         });

//         assert!(transitioned);
//         assert_eq!(c.status, Status::Blocked { initiator });
//     }

//     #[test]
//     fn should_not_transition_from_accepted_to_blocked_when_initiator_is_not_a_member() {
//         let mut c = Contact::new(&Sub::from("123"), &Sub::from("456"));
//         c.set_status(Status::Accepted);

//         let transitioned = c.transition(StatusTransition::Block {
//             initiator: &Sub::from("789"),
//         });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Accepted);
//     }

//     #[test]
//     fn should_transition_from_blocked_to_accepted() {
//         let initiator = Sub::from("123");
//         let target = &Sub::from("456");
//         let mut c = Contact::new(&initiator, target);
//         c.set_status(Status::Blocked { initiator });

//         let transitioned = c.transition(StatusTransition::Unblock { target });

//         assert!(transitioned);
//         assert_eq!(c.status, Status::Accepted);
//     }

//     #[test]
//     fn should_not_transition_from_blocked_to_accepted_when_target_is_not_a_member() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &Sub::from("456"));
//         c.set_status(Status::Blocked {
//             initiator: initiator.clone(),
//         });

//         let transitioned = c.transition(StatusTransition::Unblock {
//             target: &Sub::from("789"),
//         });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Blocked { initiator });
//     }

//     #[test]
//     fn should_not_transition_from_blocked_to_accepted_when_target_is_initiator() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &Sub::from("456"));
//         c.set_status(Status::Blocked {
//             initiator: initiator.clone(),
//         });

//         let transitioned = c.transition(StatusTransition::Unblock { target: &initiator });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Blocked { initiator });
//     }

//     #[test]
//     fn should_not_block_pending() {
//         let initiator = Sub::from("123");
//         let mut c = Contact::new(&initiator, &Sub::from("456"));

//         let transitioned = c.transition(StatusTransition::Block {
//             initiator: &initiator,
//         });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Pending { initiator });
//     }

//     #[test]
//     fn should_not_unblock_pending() {
//         let initiator = Sub::from("123");
//         let target = &Sub::from("456");
//         let mut c = Contact::new(&initiator, target);

//         let transitioned = c.transition(StatusTransition::Unblock { target });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Pending { initiator });
//     }

//     #[test]
//     fn should_not_accept_accepted() {
//         let responder = &Sub::from("456");
//         let mut c = Contact::new(&Sub::from("123"), responder);
//         c.set_status(Status::Accepted);

//         let transitioned = c.transition(StatusTransition::Accept { responder });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Accepted);
//     }

//     #[test]
//     fn should_not_reject_accepted() {
//         let responder = &Sub::from("456");
//         let mut c = Contact::new(&Sub::from("123"), responder);
//         c.set_status(Status::Accepted);

//         let transitioned = c.transition(StatusTransition::Reject { responder });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Accepted);
//     }

//     #[test]
//     fn should_not_unblock_accepted() {
//         let target = &Sub::from("123");
//         let mut c = Contact::new(target, &Sub::from("456"));
//         c.set_status(Status::Accepted);

//         let transitioned = c.transition(StatusTransition::Unblock { target });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Accepted);
//     }

//     #[test]
//     fn should_not_accept_blocked() {
//         let initiator = Sub::from("123");
//         let responder = &Sub::from("456");
//         let mut c = Contact::new(&initiator, responder);
//         c.set_status(Status::Blocked {
//             initiator: initiator.clone(),
//         });

//         let transitioned = c.transition(StatusTransition::Accept { responder });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Blocked { initiator });
//     }

//     #[test]
//     fn should_not_reject_blocked() {
//         let initiator = Sub::from("123");
//         let responder = &Sub::from("456");
//         let mut c = Contact::new(&initiator, responder);
//         c.set_status(Status::Blocked {
//             initiator: initiator.clone(),
//         });

//         let transitioned = c.transition(StatusTransition::Reject { responder });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Blocked { initiator });
//     }

//     #[test]
//     fn should_not_block_blocked() {
//         let initiator = Sub::from("123");
//         let target = &Sub::from("456");
//         let mut c = Contact::new(&initiator, target);
//         c.set_status(Status::Blocked {
//             initiator: initiator.clone(),
//         });

//         let transitioned = c.transition(StatusTransition::Block { initiator: target });

//         assert!(!transitioned);
//         assert_eq!(c.status, Status::Blocked { initiator });
//     }
// }
