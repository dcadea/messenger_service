use std::future::Future;

use warp::http::StatusCode;
use warp::Reply;

pub fn health_handler() -> impl Future<Output = crate::Result<impl Reply>> {
    futures::future::ready(Ok(StatusCode::OK))
}
