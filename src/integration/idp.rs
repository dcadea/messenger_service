#[derive(Clone)]
pub struct Config {
    pub issuer: String,
    pub jwks_url: String,
    pub userinfo_url: String,
    pub audience: Vec<String>,
    pub required_claims: Vec<String>,
}

impl Config {
    pub fn new(issuer: String, audience: Vec<String>, required_claims: Vec<String>) -> Self {
        Self {
            jwks_url: format!("{}.well-known/jwks.json", &issuer),
            userinfo_url: format!("{}userinfo", &issuer),
            issuer,
            audience,
            required_claims,
        }
    }
}
