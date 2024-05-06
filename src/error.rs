use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use serde::Serialize;

#[derive(Debug)]
pub(crate) enum ApiError {
    InternalServerError(String),

    QueryParamRequired(String),

    Unauthorized,
    Forbidden,
    TokenMalformed(String),

    ParseError(serde_json::Error),
    ReqwestError(reqwest::Error),

    RabbitMQError(lapin::Error),
    MongoDBError(mongodb::error::Error),
    RedisError(redis::RedisError),
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        Self::ParseError(error)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        Self::ReqwestError(error)
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
            Self::QueryParamRequired(param) => (
                StatusCode::BAD_REQUEST,
                format!("Query parameter '{}' is required", param),
            ),

            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_owned()),
            Self::TokenMalformed(message) => {
                error!("Token malformed: {:?}", message);
                (StatusCode::BAD_REQUEST, "Token malformed".to_owned())
            }

            internal => {
                match internal {
                    Self::InternalServerError(message) => {
                        error!("Internal server error: {:?}", message)
                    }
                    Self::ParseError(error) => error!("Parse error: {:?}", error),
                    Self::ReqwestError(error) => error!("Reqwest error: {:?}", error),
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
