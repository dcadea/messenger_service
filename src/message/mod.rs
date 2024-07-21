use crate::message::model::MessageId;
use thiserror::Error;

pub mod api;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, MessageError>;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum MessageError {
    #[error("message not found: {0:?}")]
    NotFound(Option<MessageId>),
    #[error("unexpected message error: {0}")]
    Unexpected(String),

    _MongoDBError(#[from] mongodb::error::Error),
}
