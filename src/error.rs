use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use serde::Serialize;
use std::borrow::ToOwned;
use thiserror::Error;

use super::auth::error::AuthError;
use super::chat::error::ChatError;
use super::event::error::EventError;
use super::integration::error::IntegrationError;
use super::message::error::MessageError;
use super::user::error::UserError;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum ApiError {
    #[error("Query parameter '{0}' is required")]
    QueryParamRequired(String),

    _AuthError(#[from] AuthError),
    _ChatError(#[from] ChatError),
    _EventError(#[from] EventError),
    _IntegrationError(#[from] IntegrationError),
    _MessageError(#[from] MessageError),
    _UserError(#[from] UserError),
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

            Self::_ChatError(ChatError::NotFound(_)) => (StatusCode::NOT_FOUND, message),

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
