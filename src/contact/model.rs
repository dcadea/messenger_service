use serde::{Deserialize, Serialize};

use crate::user;

use super::{Id, Status, StatusTransition};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct Contact {
    #[serde(rename = "_id")]
    id: Id,
    sub1: user::Sub,
    sub2: user::Sub,
    status: Status,
}

impl Contact {
    pub fn new(initiator: &user::Sub, responder: &user::Sub) -> Self {
        Self {
            id: Id::random(),
            sub1: initiator.clone(),
            sub2: responder.clone(),
            status: Status::Pending {
                initiator: initiator.clone(),
            },
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn sub1(&self) -> &user::Sub {
        &self.sub1
    }

    pub fn sub2(&self) -> &user::Sub {
        &self.sub2
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Possible transitions:
    /// - Pending -> (Accept) -> Accepted
    /// - Pending -> (Reject) -> Rejected
    /// - Accepted -> (Block) -> Blocked
    /// - Blocked -> (Unblock) -> Accepted
    pub fn transition(&mut self, t: StatusTransition) -> bool {
        let mut changed = true;

        match (&self.status, t) {
            (Status::Pending { initiator }, StatusTransition::Accept { responder }) => {
                if !self.is_member(&responder) {
                    return false;
                }

                if initiator.eq(&responder) {
                    return false;
                }

                self.status = Status::Accepted;
            }
            (Status::Pending { initiator }, StatusTransition::Reject { responder }) => {
                if !self.is_member(&responder) {
                    return false;
                }

                if initiator.eq(&responder) {
                    return false;
                }

                self.status = Status::Rejected;
            }
            (Status::Accepted, StatusTransition::Block { initiator }) => {
                if !self.is_member(&initiator) {
                    return false;
                }

                self.status = Status::Blocked { initiator }
            }
            (Status::Blocked { initiator }, StatusTransition::Unblock { target }) => {
                if !self.is_member(&target) {
                    return false;
                }

                if initiator.eq(&target) {
                    return false;
                }

                self.status = Status::Accepted;
            }
            (_, _) => {
                changed = false; /* no change */
            }
        }

        changed
    }

    fn is_member(&self, sub: &user::Sub) -> bool {
        if self.sub1.eq(sub) {
            return true;
        }

        if self.sub2.eq(sub) {
            return true;
        }

        false
    }
}

pub struct ContactDto {
    id: Id,
    recipient: user::Sub,
    status: Status,
}

impl ContactDto {
    pub fn new(id: Id, recipient: user::Sub, status: Status) -> Self {
        Self {
            id,
            recipient,
            status,
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn recipient(&self) -> &user::Sub {
        &self.recipient
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self.status, Status::Accepted)
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.status, Status::Pending { .. })
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self.status, Status::Rejected)
    }
}

#[cfg(test)]
impl Contact {
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }
}

#[cfg(test)]
mod test {
    use crate::{
        contact::{Status, StatusTransition},
        user,
    };

    use super::Contact;

    #[test]
    fn should_create_in_pending_state() {
        let initiator = user::Sub("123".into());
        let c = Contact::new(&initiator, &user::Sub("456".into()));

        assert_eq!(c.status, Status::Pending { initiator });
    }

    #[test]
    fn should_transition_from_pending_to_accepted() {
        let responder = user::Sub("456".into());
        let mut c = Contact::new(&user::Sub("123".into()), &responder);

        let transitioned = c.transition(StatusTransition::Accept { responder });

        assert!(transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_transition_from_pending_to_accepted_when_responder_not_a_member() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Accept {
            responder: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Pending { initiator });
    }

    #[test]
    fn should_not_transition_from_pending_to_accepted_when_same_subs() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &initiator);

        let transitioned = c.transition(StatusTransition::Accept {
            responder: initiator.clone(),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Pending { initiator });
    }

    #[test]
    fn should_transition_from_pending_to_rejected() {
        let responder = user::Sub("456".into());
        let mut c = Contact::new(&user::Sub("123".into()), &responder);

        let transitioned = c.transition(StatusTransition::Reject { responder });

        assert!(transitioned);
        assert_eq!(c.status, Status::Rejected);
    }

    #[test]
    fn should_not_transition_from_pending_to_rejected_when_responder_not_a_member() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Reject {
            responder: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Pending { initiator });
    }

    #[test]
    fn should_not_transition_from_pending_to_rejected_when_same_subs() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &initiator);

        let transitioned = c.transition(StatusTransition::Reject {
            responder: initiator.clone(),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Pending { initiator });
    }

    #[test]
    fn should_transition_from_accepted_to_blocked_when_initiator_is_sub1() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &user::Sub("456".into()));
        c.set_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Block {
            initiator: initiator.clone(),
        });

        assert!(transitioned);
        assert_eq!(c.status, Status::Blocked { initiator });
    }

    #[test]
    fn should_transition_from_accepted_to_blocked_when_initiator_is_sub2() {
        let initiator = user::Sub("456".into());
        let mut c = Contact::new(&user::Sub("123".into()), &initiator);
        c.set_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Block {
            initiator: initiator.clone(),
        });

        assert!(transitioned);
        assert_eq!(c.status, Status::Blocked { initiator });
    }

    #[test]
    fn should_not_transition_from_accepted_to_blocked_when_initiator_is_not_a_member() {
        let mut c = Contact::new(&user::Sub("123".into()), &user::Sub("456".into()));
        c.set_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Block {
            initiator: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_transition_from_blocked_to_accepted() {
        let initiator = user::Sub("123".into());
        let target = user::Sub("456".into());
        let mut c = Contact::new(&initiator, &target);
        c.set_status(Status::Blocked { initiator });

        let transitioned = c.transition(StatusTransition::Unblock { target });

        assert!(transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_transition_from_blocked_to_accepted_when_target_is_not_a_member() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &user::Sub("456".into()));
        c.set_status(Status::Blocked {
            initiator: initiator.clone(),
        });

        let transitioned = c.transition(StatusTransition::Unblock {
            target: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Blocked { initiator });
    }

    #[test]
    fn should_not_transition_from_blocked_to_accepted_when_target_is_initiator() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &user::Sub("456".into()));
        c.set_status(Status::Blocked {
            initiator: initiator.clone(),
        });

        let transitioned = c.transition(StatusTransition::Unblock {
            target: initiator.clone(),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Blocked { initiator });
    }

    #[test]
    fn should_not_block_pending() {
        let initiator = user::Sub("123".into());
        let mut c = Contact::new(&initiator, &user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Block {
            initiator: initiator.clone(),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Pending { initiator });
    }

    #[test]
    fn should_not_unblock_pending() {
        let initiator = user::Sub("123".into());
        let target = user::Sub("456".into());
        let mut c = Contact::new(&initiator, &target);

        let transitioned = c.transition(StatusTransition::Unblock { target });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Pending { initiator });
    }

    #[test]
    fn should_not_accept_accepted() {
        let responder = user::Sub("456".into());
        let mut c = Contact::new(&user::Sub("123".into()), &responder);
        c.set_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Accept { responder });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_reject_accepted() {
        let responder = user::Sub("456".into());
        let mut c = Contact::new(&user::Sub("123".into()), &responder);
        c.set_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Reject { responder });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_unblock_accepted() {
        let target = user::Sub("123".into());
        let mut c = Contact::new(&target, &user::Sub("456".into()));
        c.set_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Unblock { target });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_accept_blocked() {
        let initiator = user::Sub("123".into());
        let responder = user::Sub("456".into());
        let mut c = Contact::new(&initiator, &responder);
        c.set_status(Status::Blocked {
            initiator: initiator.clone(),
        });

        let transitioned = c.transition(StatusTransition::Accept { responder });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Blocked { initiator });
    }

    #[test]
    fn should_not_reject_blocked() {
        let initiator = user::Sub("123".into());
        let responder = user::Sub("456".into());
        let mut c = Contact::new(&initiator, &responder);
        c.set_status(Status::Blocked {
            initiator: initiator.clone(),
        });

        let transitioned = c.transition(StatusTransition::Reject { responder });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Blocked { initiator });
    }

    #[test]
    fn should_not_block_blocked() {
        let initiator = user::Sub("123".into());
        let target = user::Sub("456".into());
        let mut c = Contact::new(&initiator, &target);
        c.set_status(Status::Blocked {
            initiator: initiator.clone(),
        });

        let transitioned = c.transition(StatusTransition::Block { initiator: target });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Blocked { initiator });
    }
}
