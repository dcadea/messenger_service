use thiserror::Error;

use super::model::ChatId;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum ChatError {
    #[error("chat not found: {0:?}")]
    NotFound(Option<ChatId>),
    #[error("unexpected chat error: {0}")]
    Unexpected(String),

    MongoDBError(#[from] mongodb::error::Error),
}
