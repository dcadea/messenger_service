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
    pub fn new(initiator: user::Sub, responder: user::Sub) -> Self {
        Self {
            id: None,
            sub1: initiator.clone(),
            sub2: responder,
            status: Status::Pending { initiator },
        }
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
        };

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
    pub id: Id,
    pub recipient: user::Sub,
    pub status: Status,
}

#[cfg(test)]
mod test {
    use crate::{
        contact::{Status, StatusTransition},
        user,
    };

    use super::Contact;

    impl Contact {
        fn with_status(&mut self, status: Status) {
            self.status = status;
        }
    }

    #[test]
    fn should_create_in_pending_state() {
        let c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));

        assert_eq!(
            c.status,
            Status::Pending {
                initiator: user::Sub("123".into())
            }
        );
    }

    #[test]
    fn should_transition_from_pending_to_accepted() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Accept {
            responder: user::Sub("456".into()),
        });

        assert!(transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_transition_from_pending_to_accepted_when_responder_not_a_member() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Accept {
            responder: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Pending {
                initiator: user::Sub("123".into())
            }
        );
    }

    #[test]
    fn should_not_transition_from_pending_to_accepted_when_same_subs() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("123".into()));

        let transitioned = c.transition(StatusTransition::Accept {
            responder: user::Sub("123".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Pending {
                initiator: user::Sub("123".into())
            }
        );
    }

    #[test]
    fn should_transition_from_pending_to_rejected() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Reject {
            responder: user::Sub("456".into()),
        });

        assert!(transitioned);
        assert_eq!(c.status, Status::Rejected);
    }

    #[test]
    fn should_not_transition_from_pending_to_rejected_when_responder_not_a_member() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Reject {
            responder: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Pending {
                initiator: user::Sub("123".into())
            }
        );
    }

    #[test]
    fn should_not_transition_from_pending_to_rejected_when_same_subs() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("123".into()));

        let transitioned = c.transition(StatusTransition::Reject {
            responder: user::Sub("123".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Pending {
                initiator: user::Sub("123".into())
            }
        );
    }

    #[test]
    fn should_transition_from_accepted_to_blocked_when_initiator_is_sub1() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Block {
            initiator: user::Sub("123".into()),
        });

        assert!(transitioned);
        assert_eq!(
            c.status,
            Status::Blocked {
                initiator: user::Sub("123".into())
            }
        );
    }

    #[test]
    fn should_transition_from_accepted_to_blocked_when_initiator_is_sub2() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Block {
            initiator: user::Sub("456".into()),
        });

        assert!(transitioned);
        assert_eq!(
            c.status,
            Status::Blocked {
                initiator: user::Sub("456".into())
            }
        );
    }

    #[test]
    fn should_not_transition_from_accepted_to_blocked_when_initiator_is_not_a_member() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Block {
            initiator: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_transition_from_blocked_to_accepted() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Blocked {
            initiator: user::Sub("123".into()),
        });

        let transitioned = c.transition(StatusTransition::Unblock {
            target: user::Sub("456".into()),
        });

        assert!(transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_transition_from_blocked_to_accepted_when_target_is_not_a_member() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Blocked {
            initiator: user::Sub("123".into()),
        });

        let transitioned = c.transition(StatusTransition::Unblock {
            target: user::Sub("789".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Blocked {
                initiator: user::Sub("123".into()),
            }
        );
    }

    #[test]
    fn should_not_transition_from_blocked_to_accepted_when_target_is_initiator() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Blocked {
            initiator: user::Sub("123".into()),
        });

        let transitioned = c.transition(StatusTransition::Unblock {
            target: user::Sub("123".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Blocked {
                initiator: user::Sub("123".into()),
            }
        );
    }

    #[test]
    fn should_not_block_pending() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Block {
            initiator: user::Sub("123".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Pending {
                initiator: user::Sub("123".into()),
            }
        );
    }

    #[test]
    fn should_not_unblock_pending() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));

        let transitioned = c.transition(StatusTransition::Unblock {
            target: user::Sub("456".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Pending {
                initiator: user::Sub("123".into()),
            }
        );
    }

    #[test]
    fn should_not_accept_accepted() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Accept {
            responder: user::Sub("456".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_reject_accepted() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Reject {
            responder: user::Sub("456".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_unblock_accepted() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Accepted);

        let transitioned = c.transition(StatusTransition::Unblock {
            target: user::Sub("123".into()),
        });

        assert!(!transitioned);
        assert_eq!(c.status, Status::Accepted);
    }

    #[test]
    fn should_not_accept_blocked() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Blocked {
            initiator: user::Sub("123".into()),
        });

        let transitioned = c.transition(StatusTransition::Accept {
            responder: user::Sub("456".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Blocked {
                initiator: user::Sub("123".into()),
            }
        );
    }

    #[test]
    fn should_not_reject_blocked() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Blocked {
            initiator: user::Sub("123".into()),
        });

        let transitioned = c.transition(StatusTransition::Reject {
            responder: user::Sub("456".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Blocked {
                initiator: user::Sub("123".into()),
            }
        );
    }

    #[test]
    fn should_not_block_blocked() {
        let mut c = Contact::new(user::Sub("123".into()), user::Sub("456".into()));
        c.with_status(Status::Blocked {
            initiator: user::Sub("123".into()),
        });

        let transitioned = c.transition(StatusTransition::Block {
            initiator: user::Sub("456".into()),
        });

        assert!(!transitioned);
        assert_eq!(
            c.status,
            Status::Blocked {
                initiator: user::Sub("123".into()),
            }
        );
    }
}
