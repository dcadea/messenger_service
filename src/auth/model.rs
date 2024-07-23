use serde::Deserialize;

use crate::user::model::Sub;

#[derive(Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: Sub,
}
