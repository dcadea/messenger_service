use std::sync::Arc;

use axum::{
    Router,
    http::StatusCode,
    routing::{delete, post},
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

#[derive(Clone, Deserialize, Serialize)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/contacts", post(handler::api::create))
        .route("/contacts/{id}", delete(handler::api::delete))
        .with_state(s)
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Pending,
    Accepted,
    Rejected,
    Blocked,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
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
            Error::AlreadyExists(..) => StatusCode::CONFLICT,
            Error::SelfReference | Error::SameSubs(_) => StatusCode::BAD_REQUEST,
            Error::_MongoDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
