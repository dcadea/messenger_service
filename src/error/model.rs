use warp::reject::Reject;

#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    message: String,
}

impl ApiError {
    pub fn new(message: &str) -> Self {
        Self { message: message.to_string() }
    }
}

impl Reject for ApiError {}