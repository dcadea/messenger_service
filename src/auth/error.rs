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
    #[error("token is malformed: {0}")]
    TokenMalformed(String),
    #[error("unexpected auth error: {0}")]
    Unexpected(String),

    UserError(#[from] UserError),
    IntegrationError(#[from] IntegrationError),
    ReqwestError(#[from] reqwest::Error),
    ParseJsonError(#[from] serde_json::Error),
}
