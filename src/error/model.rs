use warp::reject::Reject;

#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    code: u16,
    message: String,
}

impl ApiError {
    pub fn new(code: u16, message: &str) -> Self {
        Self { code, message: message.to_string() }
    }
}

impl Reject for ApiError {}