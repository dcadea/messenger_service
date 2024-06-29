use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum IntegrationError {
    MongoDB(#[from] mongodb::error::Error),
    Lapin(#[from] lapin::Error),
    Redis(#[from] redis::RedisError),
    Reqwest(#[from] reqwest::Error),
}
