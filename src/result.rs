use crate::error::ApiError;

pub(crate) type Result<T> = std::result::Result<T, ApiError>;
