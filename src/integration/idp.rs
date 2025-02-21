use std::{sync::Arc, time::Duration};

use oauth2::{
    AuthUrl, ClientId, ClientSecret, RedirectUrl, RevocationUrl, TokenUrl, basic::BasicClient,
};

#[derive(Clone)]
pub struct Config {
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    revocation_url: String,
    redirect_url: String,
    pub userinfo_url: String,
    pub jwks_url: String,
    pub issuer: String,
    pub audience: String,
    pub required_claims: Arc<[String]>,
    pub token_ttl: Duration,
}

impl Config {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_url: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        required_claims: &[String],
        token_ttl: Duration,
    ) -> Self {
        let issuer = issuer.into();
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            auth_url: format!("{issuer}authorize"),
            token_url: format!("{issuer}oauth/token"),
            revocation_url: format!("{issuer}oauth/revoke"),
            redirect_url: redirect_url.into(),
            userinfo_url: format!("{issuer}userinfo"),
            jwks_url: format!("{issuer}.well-known/jwks.json"),
            issuer,
            audience: audience.into(),
            required_claims: required_claims.into(),
            token_ttl,
        }
    }
}

pub fn init(config: &Config) -> BasicClient {
    let client_id = ClientId::new(config.client_id.to_owned());
    let client_secret = ClientSecret::new(config.client_secret.to_owned());
    let auth_url = AuthUrl::new(config.auth_url.to_owned()).expect("Invalid authorization URL");
    let token_url = TokenUrl::new(config.token_url.to_owned()).expect("Invalid token URL");
    let redirect_url =
        RedirectUrl::new(config.redirect_url.to_owned()).expect("Invalid redirect URL");

    let revocation_url =
        RevocationUrl::new(config.revocation_url.to_owned()).expect("Invalid revocation URL");

    BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        .set_redirect_uri(redirect_url)
        .set_revocation_uri(revocation_url)
}
