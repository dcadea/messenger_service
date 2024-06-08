use thiserror::Error;

use super::model::MessageId;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum MessageError {
    #[error("message not found: {0:?}")]
    NotFound(Option<MessageId>),
    #[error("unexpected message error: {0}")]
    Unexpected(String),

    MongoDBError(#[from] mongodb::error::Error),
}
