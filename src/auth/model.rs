use serde::Deserialize;

use crate::user::model::Sub;

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: String,
    pub state: String, // TODO: use state
}

#[derive(Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: Sub,
}
