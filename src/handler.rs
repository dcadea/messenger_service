use std::future::{ready, Future};

use warp::http::StatusCode;
use warp::{Rejection, Reply};

pub fn health_handler() -> impl Future<Output = Result<impl Reply, Rejection>> {
    ready(Ok(StatusCode::OK))
}
