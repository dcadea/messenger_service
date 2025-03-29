use std::sync::Arc;

use axum::http::StatusCode;
use repository::ContactRepository;
use serde::{Deserialize, Serialize};

use mongodb::bson::serde_helpers::hex_string_as_object_id;
use service::ContactService;

use crate::user;

pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn ContactRepository + Send + Sync>;
pub type Service = Arc<dyn ContactService + Send + Sync>;

#[derive(Clone, Deserialize, Serialize)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

#[derive(Serialize, Deserialize, Clone)]
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

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}

impl From<Error> for StatusCode {
    fn from(e: Error) -> Self {
        match e {
            Error::AlreadyExists(..) => StatusCode::CONFLICT,
            Error::_MongoDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
