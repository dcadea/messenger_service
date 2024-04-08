use std::future::Future;

use warp::http::StatusCode;
use warp::{Rejection, Reply};

pub fn health_handler() -> impl Future<Output = Result<impl Reply, Rejection>> {
    futures::future::ready(Ok(StatusCode::OK))
}
