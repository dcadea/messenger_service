use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use serde::Serialize;

#[derive(Debug)]
pub(crate) enum ApiError {
    BadRequest(String),

    UserAlreadyExists,
    UserNotFound,
    InvalidCredentials,

    OpenIDError(openid::error::Error),
    OpenIDClientError(openid::error::ClientError),

    ParseError(url::ParseError),
    WebSocketConnectionRejected,

    RabbitMQError(lapin::Error),
    MongoDBError(mongodb::error::Error),
    RedisError(redis::RedisError),
}

impl From<openid::error::Error> for ApiError {
    fn from(error: openid::error::Error) -> Self {
        Self::OpenIDError(error)
    }
}

impl From<openid::error::ClientError> for ApiError {
    fn from(error: openid::error::ClientError) -> Self {
        Self::OpenIDClientError(error)
    }
}

impl From<url::ParseError> for ApiError {
    fn from(error: url::ParseError) -> Self {
        Self::ParseError(error)
    }
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

impl From<redis::RedisError> for ApiError {
    fn from(error: redis::RedisError) -> Self {
        Self::RedisError(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),

            Self::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists".to_owned()),
            Self::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_owned()),
            Self::InvalidCredentials => (StatusCode::FORBIDDEN, "Invalid credentials".to_owned()),

            internal => {
                match internal {
                    Self::OpenIDError(error) => error!("OpenID error: {:?}", error),
                    Self::OpenIDClientError(error) => error!("OpenID client error: {:?}", error),
                    Self::ParseError(error) => error!("Parse error: {:?}", error),
                    Self::WebSocketConnectionRejected => error!("WebSocket connection rejected"),
                    Self::RabbitMQError(error) => error!("RabbitMQ error: {:?}", error),
                    Self::MongoDBError(error) => error!("MongoDB error: {:?}", error),
                    Self::RedisError(error) => error!("Redis error: {:?}", error),
                    _ => {}
                }
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
        };

        (status, Json(ErrorResponse { message })).into_response()
    }
}
