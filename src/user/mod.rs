use model::Sub;

pub mod markup;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("user not found: {:?}", 0)]
    NotFound(Sub),

    _MongoDB(#[from] mongodb::error::Error),
    _Redis(#[from] redis::RedisError),
    _ParseJson(#[from] serde_json::Error),
}
