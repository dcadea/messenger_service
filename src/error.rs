use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::debug;
use serde::Serialize;

#[derive(Debug)]
pub(crate) enum ApiError {
    UserAlreadyExists,
    UserNotFound,

    InvalidCredentials,

    WebSocketConnectionRejected,

    RabbitMQError(lapin::Error),
    MongoDBError(mongodb::error::Error),
}

impl From<lapin::Error> for ApiError {
    fn from(error: lapin::Error) -> Self {
        Self::RabbitMQError(error)
    }
}

impl From<mongodb::error::Error> for ApiError {
    fn from(error: mongodb::error::Error) -> Self {
        Self::MongoDBError(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            Self::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists".to_owned()),
            Self::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_owned()),

            Self::InvalidCredentials => (StatusCode::FORBIDDEN, "Invalid credentials".to_owned()),

            Self::WebSocketConnectionRejected => {
                (StatusCode::FORBIDDEN, "WS connection rejected".to_owned())
            }

            Self::RabbitMQError(e) => {
                debug!("RabbitMQ error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }

            Self::MongoDBError(e) => {
                debug!("MongoDB error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
        };

        (status, Json(ErrorResponse { message })).into_response()
    }
}
