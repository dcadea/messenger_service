use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum IntegrationError {
    MongoDBError(#[from] mongodb::error::Error),
    RabbitMQError(#[from] lapin::Error),
    RedisError(#[from] redis::RedisError),
    ReqwestError(#[from] reqwest::Error),
}
