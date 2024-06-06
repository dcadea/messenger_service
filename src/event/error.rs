use crate::auth::error::AuthError;
use crate::chat::error::ChatError;
use crate::message::error::MessageError;
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum EventError {
    #[error("Missing user info")]
    MissingUserInfo,

    AuthError(#[from] AuthError),
    ChatError(#[from] ChatError),
    MessageError(#[from] MessageError),

    ParseJsonError(#[from] serde_json::Error),
    RabbitMQError(#[from] lapin::Error),
}
