use thiserror::Error;

use crate::integration::error::IntegrationError;
use crate::user::error::UserError;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum AuthError {
    #[error("unauthorized to access the resource")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("missing or unknown kid")]
    UnknownKid,
    #[error("token is malformed: {0}")]
    TokenMalformed(String),
    #[error("unexpected auth error: {0}")]
    Unexpected(String),

    _UserError(#[from] UserError),
    _IntegrationError(#[from] IntegrationError),
    _ReqwestError(#[from] reqwest::Error),
    _ParseJsonError(#[from] serde_json::Error),
}
