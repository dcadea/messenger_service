use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: String,
}
