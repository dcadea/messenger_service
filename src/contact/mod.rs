use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, post, put},
};
use repository::ContactRepository;
use serde::{Deserialize, Serialize};

use mongodb::bson::serde_helpers::hex_string_as_object_id;
use service::ContactService;

use crate::{state::AppState, user};

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn ContactRepository + Send + Sync>;
pub type Service = Arc<dyn ContactService + Send + Sync>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

impl Id {
    pub fn random() -> Self {
        Self(mongodb::bson::oid::ObjectId::new().to_hex())
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
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }

    pub const fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected)
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
    #[error("contact ({0:?} : {1:?}) already exists")]
    AlreadyExists(user::Id, user::Id),
    #[error("contacts should be different, got both: {0:?}")]
    SameUsers(user::Id),
    #[error("could not transition contact status")]
    StatusTransitionFailed,

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}
