use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use serde::Serialize;

use crate::{auth, chat, event, message, user};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Query parameter '{0}' is required")]
    QueryParamRequired(String),

    #[error(transparent)]
    _Auth(#[from] auth::Error),
    #[error(transparent)]
    _Chat(#[from] chat::Error),
    #[error(transparent)]
    _Event(#[from] event::Error),
    #[error(transparent)]
    _Message(#[from] message::Error),
    #[error(transparent)]
    _User(#[from] user::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let error_message = self.to_string();

        let (status, message) = match self {
            Self::_Auth(auth) => return auth.into_response(),
            Self::_Chat(chat) => return chat.into_response(),

            Self::_Event(event::Error::NotOwner) => (StatusCode::FORBIDDEN, error_message),
            Self::_Event(event::Error::NotRecipient) => (StatusCode::FORBIDDEN, error_message),

            Self::_Message(message::Error::NotFound(_)) => (StatusCode::NOT_FOUND, error_message),
            Self::_Message(message::Error::NotOwner) => (StatusCode::BAD_REQUEST, error_message),

            Self::_User(user::Error::NotFound(_)) => (StatusCode::NOT_FOUND, error_message),

            Self::QueryParamRequired(_) => (StatusCode::BAD_REQUEST, error_message),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_owned(),
            ),
        };

        error!("{self}");

        (status, Json(ErrorResponse { message })).into_response()
    }
}
