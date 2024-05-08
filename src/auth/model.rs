use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct UserInfo {
    pub sub: String,
    pub nickname: String,
    pub name: String,
    pub picture: String,
    pub email: String,
}

#[derive(Deserialize, Clone)]
pub(super) struct TokenClaims {
    pub sub: String,
}
