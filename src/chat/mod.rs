use crate::chat::model::ChatId;
use crate::user;
use crate::user::model::UserSub;

pub mod api;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("chat not found: {0:?}")]
    NotFound(Option<ChatId>),
    #[error("chat already exists for members: {0:?}")]
    AlreadyExists([UserSub; 2]),
    #[error("user is not a member of the chat")]
    NotMember,
    #[error("unexpected chat error: {0}")]
    Unexpected(String),

    _User(#[from] user::Error),

    _MongoDB(#[from] mongodb::error::Error),
    _Redis(#[from] redis::RedisError),
}
