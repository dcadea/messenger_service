use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub(super) struct TokenClaims {
    pub sub: String,
}
