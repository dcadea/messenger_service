use crate::error::ApiError;

pub type Result<T> = std::result::Result<T, ApiError>;
