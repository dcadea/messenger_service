use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, post, put},
};
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types};
use model::Contact;
use repository::ContactRepository;
use serde::{Deserialize, Serialize};

use service::ContactService;
use uuid::Uuid;

use crate::{state::AppState, user};

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn ContactRepository + Send + Sync>;
pub type Service = Arc<dyn ContactService + Send + Sync>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Uuid)]
pub struct Id(Uuid);

impl Id {
    pub fn get(&self) -> &Uuid {
        &self.0
    }
}

impl From<Uuid> for Id {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/contacts", post(handler::api::create))
        .route("/contacts/{id}", delete(handler::api::delete))
        .route("/contacts/{id}/{transition}", put(handler::api::transition))
        .with_state(s)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(tag = "indicator", rename_all = "snake_case")]
pub enum Status {
    Pending { initiator: user::Id },
    Accepted,
    Rejected,
    Blocked { initiator: user::Id },
}

impl Status {
    pub const fn as_str(&self) -> &str {
        match self {
            Status::Pending { .. } => "pending",
            Status::Accepted => "accepted",
            Status::Rejected => "rejected",
            Status::Blocked { .. } => "blocked",
        }
    }

    pub const fn initiator(&self) -> Option<&user::Id> {
        match self {
            Status::Pending { initiator } => Some(&initiator),
            Status::Accepted => None,
            Status::Rejected => None,
            Status::Blocked { initiator } => Some(&initiator),
        }
    }

    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }

    pub const fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected)
    }
}

impl From<&Contact> for Status {
    fn from(c: &Contact) -> Self {
        match c.initiator().cloned() {
            Some(initiator) => {
                if c.status().eq("pending") {
                    Status::Pending { initiator }
                } else if c.status().eq("blocked") {
                    Status::Blocked { initiator }
                } else {
                    unreachable!("unsupported status")
                }
            }
            None => {
                if c.status().eq("accepted") {
                    Status::Accepted
                } else if c.status().eq("rejected") {
                    Status::Rejected
                } else {
                    unreachable!("unsupported status")
                }
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transition {
    Accept,
    Reject,
    Block,
    Unblock,
}

pub enum StatusTransition<'a> {
    Accept { responder: &'a user::Id },
    Reject { responder: &'a user::Id },
    Block { initiator: &'a user::Id },
    Unblock { target: &'a user::Id },
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("contact not found: {0:?}")]
    NotFound(Id),
    #[error("contact already exists")]
    AlreadyExists,
    #[error("contacts should be different, got both: {0:?}")]
    SameUsers(user::Id),
    #[error("could not transition contact status")]
    StatusTransitionFailed,

    #[error(transparent)]
    _R2d2(#[from] r2d2::Error),
    #[error(transparent)]
    _Diesel(#[from] diesel::result::Error),
}
