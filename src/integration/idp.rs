use std::{sync::Arc, time::Duration};

use messenger_service::Raw;
use oauth2::{
    AuthType, AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl,
    RevocationUrl, StandardRevocableToken, TokenUrl,
    basic::{
        BasicClient, BasicErrorResponse, BasicRevocationErrorResponse,
        BasicTokenIntrospectionResponse, BasicTokenResponse,
    },
};

use crate::auth;

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

pub type OAuth2Client<
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointSet,
    HasTokenUrl = EndpointSet,
> = oauth2::Client<
    BasicErrorResponse,
    BasicTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

impl Config {
    pub fn init_client(&self) -> OAuth2Client {
        BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(self.auth_url.clone()).expect("Invalid authorization URL"))
            .set_auth_type(AuthType::RequestBody)
            .set_token_uri(TokenUrl::new(self.token_url.clone()).expect("Invalid token URL"))
            .set_redirect_uri(
                RedirectUrl::new(self.redirect_url.clone()).expect("Invalid redirect URL"),
            )
            .set_revocation_url(
                RevocationUrl::new(self.revocation_url.clone()).expect("Invalid revocation URL"),
            )
    }
}

impl From<oauth2::AuthorizationCode> for auth::Code {
    fn from(c: oauth2::AuthorizationCode) -> Self {
        Self::new(c.into_secret())
    }
}

impl From<auth::Code> for oauth2::AuthorizationCode {
    fn from(c: auth::Code) -> Self {
        oauth2::AuthorizationCode::new(c.raw().to_string())
    }
}

impl From<oauth2::CsrfToken> for auth::Csrf {
    fn from(csrf: oauth2::CsrfToken) -> Self {
        Self::new(csrf.into_secret())
    }
}
