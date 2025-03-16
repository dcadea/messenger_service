use axum::{
    Router,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};

use mongodb::bson::serde_helpers::hex_string_as_object_id;

use crate::state::State;

mod handler;
mod model;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Id(#[serde(with = "hex_string_as_object_id")] pub String);

pub fn api<S>(s: State) -> Router<S> {
    Router::new()
        .route("/threads", post(handler::api::create))
        .route("/threads/{id}", delete(handler::api::delete))
        .with_state(s)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {}
