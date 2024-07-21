use crate::message::error::MessageError;

pub mod api;
pub mod error;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, MessageError>;
