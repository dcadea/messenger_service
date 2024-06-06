use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use serde::Serialize;
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
    AuthError(#[from] AuthError),
    ChatError(#[from] ChatError),
    EventError(#[from] EventError),
    IntegrationError(#[from] IntegrationError),
    MessageError(#[from] MessageError),
    UserError(#[from] UserError),

    #[error("Query parameter '{0}' is required")]
    QueryParamRequired(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            Self::AuthError(auth_error) => {
                error!("Auth error: {:?}", auth_error);
                match auth_error {
                    AuthError::Unauthorized => {
                        (StatusCode::UNAUTHORIZED, "Unauthorized".to_owned())
                    }
                    AuthError::Forbidden(_) => (StatusCode::FORBIDDEN, "Forbidden".to_owned()),
                    AuthError::TokenMalformed(_) => {
                        (StatusCode::BAD_REQUEST, "Token malformed".to_owned())
                    }
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Something went wrong".to_owned(),
                    ),
                }
            }
            Self::ChatError(chat_error) => {
                error!("Chat error: {:?}", chat_error);
                match chat_error {
                    ChatError::NotFound(_) => (StatusCode::NOT_FOUND, chat_error.to_string()),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Something went wrong".to_owned(),
                    ),
                }
            }
            Self::EventError(event_error) => {
                error!("Event error: {:?}", event_error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
            Self::IntegrationError(integration_error) => {
                error!("Integration error: {:?}", integration_error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
            Self::MessageError(message_error) => {
                error!("Message error: {:?}", message_error);
                match message_error {
                    MessageError::NotFound(_) => (StatusCode::NOT_FOUND, message_error.to_string()),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Something went wrong".to_owned(),
                    ),
                }
            }
            Self::UserError(user_error) => {
                error!("User error: {:?}", user_error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
            Self::QueryParamRequired(param) => {
                let message = Self::QueryParamRequired(param).to_string();
                error!("{}", message);
                (StatusCode::BAD_REQUEST, message)
            }
        };

        (status, Json(ErrorResponse { message })).into_response()
    }
}
