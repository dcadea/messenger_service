use crate::chat::error::ChatError;

pub mod api;
pub mod error;
pub mod model;
pub mod repository;
pub mod service;

pub type Result<T> = std::result::Result<T, ChatError>;
