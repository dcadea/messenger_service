use std::fmt::Display;

use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use log::error;
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::user;

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
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

#[derive(Clone, Serialize, Deserialize)]
pub enum Kind {
    Private,
    Group,
}

pub fn pages<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/", get(handler::pages::home))
        .route("/chats/:id", get(handler::pages::active_chat))
        .with_state(state)
}

pub fn api<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/chats/:id", get(handler::api::find_one))
        .route("/chats", post(handler::api::create))
        .route("/chats/:id", delete(handler::api::delete))
        .with_state(state)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("chat not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("user is not a member of the chat")]
    NotMember,
    #[error("could not create chat")]
    NotCreated,
    #[error("chat already exists")]
    AlreadyExists,

    #[error(transparent)]
    _User(#[from] user::Error),

    #[error(transparent)]
    _MongoDB(#[from] mongodb::error::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        error!("{self}");

        let (status, message) = match self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::NotMember => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::NotCreated => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::AlreadyExists => (StatusCode::CONFLICT, self.to_string()),

            Self::_User(_) | Self::_MongoDB(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_owned(),
            ),
        };

        (status, message).into_response()
    }
}
