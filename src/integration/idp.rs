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
    userinfo_url: String,
    jwks_url: String,
    issuer: String,
    audience: String,
    required_claims: Arc<[String]>,
    token_ttl: Duration,
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

    pub fn userinfo_url(&self) -> &str {
        &self.userinfo_url
    }

    pub fn jwks_url(&self) -> &str {
        &self.jwks_url
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn audience(&self) -> &str {
        &self.audience
    }

    pub fn required_claims(&self) -> &[String] {
        &self.required_claims
    }

    pub fn token_ttl(&self) -> Duration {
        self.token_ttl
    }
}

impl Config {
    pub fn init_client(&self) -> oauth2::basic::BasicClient {
        let client_id = ClientId::new(self.client_id.clone());
        let client_secret = ClientSecret::new(self.client_secret.clone());
        let auth_url = AuthUrl::new(self.auth_url.clone()).expect("Invalid authorization URL");
        let token_url = TokenUrl::new(self.token_url.clone()).expect("Invalid token URL");
        let redirect_url =
            RedirectUrl::new(self.redirect_url.clone()).expect("Invalid redirect URL");

        let revocation_url =
            RevocationUrl::new(self.revocation_url.clone()).expect("Invalid revocation URL");

        BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url)
            .set_revocation_uri(revocation_url)
    }
}
