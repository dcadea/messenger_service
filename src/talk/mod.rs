use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post},
};
use diesel::{deserialize::FromSqlRow, expression::AsExpression};
use log::error;
use repository::TalkRepository;
use serde::{Deserialize, Serialize};

use service::TalkService;
use uuid::Uuid;

use crate::{integration, schema::sql_types, state::AppState, user};

mod handler;
pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;
pub type Repository = Arc<dyn TalkRepository + Send + Sync>;
pub type Service = Arc<dyn TalkService + Send + Sync>;

pub fn pages<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/talks/{id}", get(handler::pages::active_talk))
        .with_state(s)
}

pub fn api<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/talks/{id}", get(handler::api::find_one))
        .route("/talks/{id}/avatar.png", get(handler::api::find_avatar))
        .route("/talks", post(handler::api::create))
        .route("/talks/{id}", delete(handler::api::delete))
        .with_state(s)
}

pub fn templates<S>(s: AppState) -> Router<S> {
    Router::new()
        .route("/talks/group/create", get(handler::templates::create_group))
        .with_state(s)
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Uuid)]
pub struct Id(Uuid);

impl Id {
    pub const fn get(&self) -> &Uuid {
        &self.0
    }
}

impl From<Uuid> for Id {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[derive(Clone, Debug, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::TalkKind)]
pub enum Kind {
    #[serde(rename = "chat")]
    Chat,
    #[serde(rename = "group")]
    Group,
}

impl Kind {
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Chat => "chat",
            Self::Group => "group",
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Picture(String);

impl Picture {
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<Id> for Picture {
    fn from(id: Id) -> Self {
        Self(format!("/api/talks/{id}/avatar.png"))
    }
}

impl From<user::Picture> for Picture {
    fn from(p: user::Picture) -> Self {
        Self(p.as_str().to_string())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("talks not found: {0:?}")]
    NotFound(Id),
    #[error("could not create talk")]
    NotCreated,
    #[error("talk already exists")]
    AlreadyExists,
    #[error("not enough members: {0:?}")]
    NotEnoughMembers(usize),
    #[error("Missing group name")]
    MissingName,
    #[error("Selected user does not exist: {0}")]
    NonExistingUser(user::Id),
    #[error("contact is missing or in non-accepted status")]
    UnsupportedStatus,
    #[error("unsupported talk kind: {0:?}")]
    UnsupportedKind(String),

    #[error(transparent)]
    _User(#[from] user::Error),
    #[error(transparent)]
    _Integration(#[from] Box<integration::Error>),
    #[error(transparent)]
    _R2d2(#[from] r2d2::Error),
    #[error(transparent)]
    _Diesel(#[from] diesel::result::Error),
}
