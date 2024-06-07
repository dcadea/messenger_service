use crate::user::error::UserError;

pub mod api;
pub mod error;
pub mod model;
pub mod repository;
pub mod service;

type Result<T> = std::result::Result<T, UserError>;
