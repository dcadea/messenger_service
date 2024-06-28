use thiserror::Error;

use super::model::{ChatId, Members};

#[derive(Error, Debug)]
#[error(transparent)]
pub enum ChatError {
    #[error("chat not found: {0:?}")]
    NotFound(Option<ChatId>),
    #[error("chat already exists for members: {0:?}")]
    AlreadyExists(Members),
    #[error("user is not a member of the chat")]
    NotMember,
    #[error("unexpected chat error: {0}")]
    Unexpected(String),

    MongoDBError(#[from] mongodb::error::Error),
    RedisError(#[from] redis::RedisError),
}
