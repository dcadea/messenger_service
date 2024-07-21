use axum::http::header::InvalidHeaderValue;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use serde::Serialize;
use std::borrow::ToOwned;
use thiserror::Error;

use super::auth::AuthError;
use super::chat::ChatError;
use super::event::EventError;
use super::integration::IntegrationError;
use super::message::MessageError;
use super::user::UserError;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum ApiError {
    #[error("Query parameter '{0}' is required")]
    QueryParamRequired(String),
    #[error("unexpected api error {0}")]
    Unexpected(String),

    _AuthError(#[from] AuthError),
    _ChatError(#[from] ChatError),
    _EventError(#[from] EventError),
    _IntegrationError(#[from] IntegrationError),
    _MessageError(#[from] MessageError),
    _UserError(#[from] UserError),
}

impl From<InvalidHeaderValue> for ApiError {
    fn from(err: InvalidHeaderValue) -> Self {
        Self::Unexpected(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let message = format!("{}", self);
        error!("{}", message);

        let (status, message) = match self {
            Self::_AuthError(AuthError::Unauthorized) => (StatusCode::UNAUTHORIZED, message),
            Self::_AuthError(AuthError::Forbidden(_)) => {
                (StatusCode::FORBIDDEN, "Forbidden".to_owned())
            }
            Self::_AuthError(AuthError::UnknownKid) => {
                (StatusCode::FORBIDDEN, "Forbidden".to_owned())
            }
            Self::_AuthError(AuthError::TokenMalformed(_)) => {
                (StatusCode::BAD_REQUEST, "Token malformed".to_owned())
            }

            Self::_EventError(EventError::MissingUserInfo) => (StatusCode::UNAUTHORIZED, message),
            Self::_EventError(EventError::NotOwner) => (StatusCode::FORBIDDEN, message),
            Self::_EventError(EventError::NotRecipient) => (StatusCode::FORBIDDEN, message),

            Self::_ChatError(ChatError::NotFound(_)) => (StatusCode::NOT_FOUND, message),
            Self::_ChatError(ChatError::AlreadyExists(_)) => (StatusCode::CONFLICT, message),
            Self::_ChatError(ChatError::NotMember) => (StatusCode::FORBIDDEN, message),

            Self::_MessageError(MessageError::NotFound(_)) => (StatusCode::NOT_FOUND, message),

            Self::_UserError(UserError::NotFound(_)) => (StatusCode::NOT_FOUND, message),

            Self::QueryParamRequired(_) => (StatusCode::BAD_REQUEST, message),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_owned(),
            ),
        };

        (status, Json(ErrorResponse { message })).into_response()
    }
}
