use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(skip)]
    _id: Option<bson::oid::ObjectId>,
    pub sub: String,
    pub nickname: String,
    pub name: String,
    pub picture: String,
    pub email: String,
}

#[derive(Deserialize, Clone)]
pub struct TokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: Vec<String>,
    pub iat: u64,
    pub exp: u64,
    pub permissions: Option<Vec<String>>,
}