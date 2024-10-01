use crate::message::model::MessageId;

pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("message not found: {0:?}")]
    NotFound(Option<MessageId>),
    #[error("unexpected message error: {0}")]
    Unexpected(String),

    _MongoDB(#[from] mongodb::error::Error),
}
