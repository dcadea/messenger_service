use std::{fmt::Display, sync::Arc};

use axum::{
    Router,
    http::StatusCode,
    routing::{delete, get, post},
};
use log::error;
use repository::TalkRepository;
use serde::{Deserialize, Serialize};

use mongodb::bson::serde_helpers::hex_string_as_object_id;
use service::{TalkService, TalkValidator};

use crate::state::AppState;

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn TalkRepository + Send + Sync>;
pub type Service = Arc<dyn TalkService + Send + Sync>;
pub type Validator = Arc<dyn TalkValidator + Send + Sync>;

pub fn pages<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/talks/{id}", get(handler::pages::active_talk))
        .with_state(s)
}

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/talks/{id}", get(handler::api::find_one))
        .route("/talks", post(handler::api::create))
        .route("/talks/{id}", delete(handler::api::delete))
        .with_state(s)
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

impl Id {
    pub fn random() -> Self {
        Self(mongodb::bson::oid::ObjectId::new().to_hex())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
pub enum Kind {
    Chat,
    Group,
}

impl Kind {
    fn as_str(&self) -> &str {
        match self {
            Kind::Chat => "chat",
            Kind::Group => "group",
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("talks not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("user is not a member of the talk")]
    NotMember,
    #[error("could not create talk")]
    NotCreated,
    #[error("could not delete talk")]
    NotDeleted,
    #[error("talk already exists")]
    AlreadyExists,
    #[error("not enough members: {0:?}")]
    NotEnoughMembers(usize),
    #[error("contact is missing or in non-accepted status")]
    UnsupportedStatus,

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::NotMember => StatusCode::FORBIDDEN,
            Error::AlreadyExists => StatusCode::CONFLICT,
            Error::NotEnoughMembers(_) | Error::UnsupportedStatus => StatusCode::BAD_REQUEST,
            Error::NotCreated | Error::NotDeleted | Error::_MongoDB(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
