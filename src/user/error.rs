use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UserError {
    MongoDBError(#[from] mongodb::error::Error),
}
