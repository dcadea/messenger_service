use std::{fmt::Display, sync::Arc};

use axum::{
    Router,
    http::StatusCode,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/contacts", post(handler::api::create))
        .route("/contacts/{id}", delete(handler::api::delete))
        .route("/contacts/{id}/accept", put(handler::api::accept))
        .route("/contacts/{id}/reject", put(handler::api::reject))
        .route("/contacts/{id}/block", put(handler::api::block))
        .with_state(s)
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Accepted,
    Rejected,
    Blocked,
}

pub enum StatusTransition {
    Accept,
    Reject,
    Block,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("contact not found: {0:?}")]
    NotFound(Id),
    #[error("contact ({0:?} : {1:?}) already exists")]
    AlreadyExists(user::Sub, user::Sub),
    #[error("cannot create contact with oneself")]
    SelfReference,
    #[error("contacts should be different, got both: {0:?}")]
    SameSubs(user::Sub),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::AlreadyExists(..) => StatusCode::CONFLICT,
            Error::SelfReference | Error::SameSubs(_) => StatusCode::BAD_REQUEST,
            Error::_MongoDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
