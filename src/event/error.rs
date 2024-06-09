use thiserror::Error;

use crate::auth::error::AuthError;
use crate::chat::error::ChatError;
use crate::message::error::MessageError;
use crate::user::error::UserError;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum EventError {
    #[error("Missing user info")]
    MissingUserInfo,

    _AuthError(#[from] AuthError),
    _ChatError(#[from] ChatError),
    _MessageError(#[from] MessageError),
    _UserError(#[from] UserError),

    _ParseJsonError(#[from] serde_json::Error),
    _RabbitMQError(#[from] lapin::Error),
}
