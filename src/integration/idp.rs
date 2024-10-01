use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

#[derive(Clone)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub userinfo_url: String,
    pub jwks_url: String,
    pub issuer: String,
    pub audience: String,
    pub required_claims: Vec<String>,
}

impl Config {
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_url: String,
        issuer: String,
        audience: String,
        required_claims: Vec<String>,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            auth_url: format!("{issuer}authorize"),
            token_url: format!("{issuer}oauth/token"),
            redirect_url,
            userinfo_url: format!("{issuer}userinfo"),
            jwks_url: format!("{issuer}.well-known/jwks.json"),
            issuer,
            audience,
            required_claims,
        }
    }
}

pub(crate) fn init(config: &Config) -> BasicClient {
    let client_id = ClientId::new(config.client_id.to_owned());
    let client_secret = ClientSecret::new(config.client_secret.to_owned());
    let auth_url = AuthUrl::new(config.auth_url.to_owned()).expect("Invalid authorization URL");
    let token_url = TokenUrl::new(config.token_url.to_owned()).expect("Invalid token URL");
    let redirect_url =
        RedirectUrl::new(config.redirect_url.to_owned()).expect("Invalid redirect URL");

    BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        .set_redirect_uri(redirect_url)
}
