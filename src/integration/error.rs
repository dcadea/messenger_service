use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum IntegrationError {
    VarError(#[from] std::env::VarError),
    ParseIntError(#[from] std::num::ParseIntError),

    MongoDB(#[from] mongodb::error::Error),
    Lapin(#[from] lapin::Error),
    Redis(#[from] redis::RedisError),
    Reqwest(#[from] reqwest::Error),
}
