use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types};
use log::error;
use repository::MessageRepository;
use serde::{Deserialize, Serialize};
use service::MessageService;
use uuid::Uuid;

use crate::{state::AppState, user};

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn MessageRepository + Send + Sync>;
pub type Service = Arc<dyn MessageService + Send + Sync>;

#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Uuid)]
pub struct Id(Uuid);

impl Id {
    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn get(&self) -> &Uuid {
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
        .route("/messages", post(handler::api::create))
        .route("/messages", get(handler::api::find_all))
        .route("/messages", put(handler::api::update))
        .route("/messages/{id}", delete(handler::api::delete))
        .with_state(s)
}

pub fn templates<S>(s: AppState) -> Router<S> {
    Router::new()
        .route(
            "/messages/input/blank",
            get(handler::templates::message_input_blank),
        )
        .route(
            "/messages/input/edit",
            get(handler::templates::message_input_edit),
        )
        .with_state(s)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("message not found: {0:?}")]
    NotFound(Option<Id>),
    #[error("message content is empty")]
    EmptyContent,

    #[error("message id not present")]
    IdNotPresent,

    #[error(transparent)]
    _User(#[from] user::Error),
    #[error(transparent)]
    _R2d2(#[from] r2d2::Error),
    #[error(transparent)]
    _Diesel(#[from] diesel::result::Error),
}
