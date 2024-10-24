use axum::http::header::InvalidHeaderValue;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use serde::Serialize;

use crate::{auth, chat, event, message, user};

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("Query parameter '{0}' is required")]
    QueryParamRequired(String),
    #[error("unexpected api error {0}")]
    Unexpected(String),

    _Auth(#[from] auth::Error),
    _Chat(#[from] chat::Error),
    _Event(#[from] event::Error),
    _Message(#[from] message::Error),
    _User(#[from] user::Error),
}

impl From<InvalidHeaderValue> for Error {
    fn from(err: InvalidHeaderValue) -> Self {
        Self::Unexpected(err.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let error_message = self.to_string();
        error!("{error_message}");

        let (status, message) = match self {
            Self::_Auth(auth::Error::Unauthorized) => (StatusCode::UNAUTHORIZED, error_message),
            Self::_Auth(auth::Error::Forbidden(_)) => {
                (StatusCode::FORBIDDEN, "Forbidden".to_owned())
            }
            Self::_Auth(auth::Error::UnknownKid) => (StatusCode::FORBIDDEN, "Forbidden".to_owned()),
            Self::_Auth(auth::Error::TokenMalformed(_)) => {
                (StatusCode::BAD_REQUEST, "Token malformed".to_owned())
            }

            Self::_Chat(chat::Error::NotFound(_)) => (StatusCode::NOT_FOUND, error_message),
            Self::_Chat(chat::Error::AlreadyExists(_)) => (StatusCode::CONFLICT, error_message),
            Self::_Chat(chat::Error::NotMember) => (StatusCode::FORBIDDEN, error_message),

            Self::_Event(event::Error::NotOwner) => (StatusCode::FORBIDDEN, error_message),
            Self::_Event(event::Error::NotRecipient) => (StatusCode::FORBIDDEN, error_message),

            Self::_Message(message::Error::NotFound(_)) => (StatusCode::NOT_FOUND, error_message),

            Self::_User(user::Error::NotFound(_)) => (StatusCode::NOT_FOUND, error_message),

            Self::QueryParamRequired(_) => (StatusCode::BAD_REQUEST, error_message),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_owned(),
            ),
        };

        (status, Json(ErrorResponse { message })).into_response()
    }
}
