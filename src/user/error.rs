use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UserError {
    #[error("user not found: {0}")]
    NotFound(String),

    _MongoDBError(#[from] mongodb::error::Error),
    _RedisError(#[from] redis::RedisError),
    _ParseJsonError(#[from] serde_json::Error),
}
