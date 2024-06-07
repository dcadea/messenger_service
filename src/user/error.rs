use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UserError {
    #[error("user not found: {0}")]
    NotFound(String),

    MongoDBError(#[from] mongodb::error::Error),
}
