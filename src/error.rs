use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::debug;
use serde::Serialize;

pub enum ApiError {
    UserAlreadyExists,
    UserNotFound,

    InvalidCredentials,

    InternalServerError,

    RabbitMQError(lapin::Error),
}

impl From<lapin::Error> for ApiError {
    fn from(error: lapin::Error) -> Self {
        Self::RabbitMQError(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            // TODO: implement error payload
            Self::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists".to_owned()),
            Self::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_owned()),

            Self::InvalidCredentials => (StatusCode::FORBIDDEN, "Invalid credentials".to_owned()),

            // TODO: remove
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),

            Self::RabbitMQError(e) => {
                debug!("RabbitMQ error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
        };

        (status, Json(ErrorResponse { message })).into_response()
    }
}
