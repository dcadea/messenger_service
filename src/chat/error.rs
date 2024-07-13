use thiserror::Error;

use crate::user::error::UserError;
use crate::user::model::UserSub;

use super::model::ChatId;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum ChatError {
    #[error("chat not found: {0:?}")]
    NotFound(Option<ChatId>),
    #[error("chat already exists for members: {0:?}")]
    AlreadyExists([UserSub; 2]),
    #[error("user is not a member of the chat")]
    NotMember,
    #[error("unexpected chat error: {0}")]
    Unexpected(String),

    _UserError(#[from] UserError),

    MongoDBError(#[from] mongodb::error::Error),
    RedisError(#[from] redis::RedisError),
}
