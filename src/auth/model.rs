use crate::user::model::UserSub;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: UserSub,
}
