use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use tokio::sync::RwLock;

use crate::integration::idp;
use crate::user::model::UserInfo;
use crate::{auth, integration};

use super::model::TokenClaims;
use super::Result;

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone)]
pub struct AuthService {
    config: Arc<idp::Config>,
    http: Arc<reqwest::Client>,
    oauth2: Arc<BasicClient>,
    jwt_validator: Arc<Validation>,
    jwk_decoding_keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
}

impl AuthService {
    pub fn try_new(config: &idp::Config) -> Result<Self> {
        let mut jwt_validator = Validation::new(jsonwebtoken::Algorithm::RS256);
        jwt_validator.set_required_spec_claims(&config.required_claims);
        jwt_validator.set_issuer(&[&config.issuer]);
        jwt_validator.set_audience(&[&config.audience]);

        let jwk_decoding_keys = Arc::new(RwLock::new(HashMap::new()));
        let service = Self {
            config: Arc::new(config.to_owned()),
            http: Arc::new(integration::init_http_client()?),
            oauth2: Arc::new(integration::idp::init(config)),
            jwt_validator: Arc::new(jwt_validator),
            jwk_decoding_keys: jwk_decoding_keys.clone(),
        };

        // TODO: uncomment
        // let http = integration::init_http_client()?;
        // let config_clone = config.clone();
        // tokio::spawn(async move {
        //     loop {
        //         match fetch_jwk_decoding_keys(&config_clone, &http).await {
        //             Ok(keys) => *jwk_decoding_keys.write().await = keys,
        //             Err(e) => eprintln!("Failed to update JWK decoding keys: {:?}", e),
        //         }
        //         sleep(ONE_DAY).await;
        //     }
        // });

        Ok(service)
    }
}

impl AuthService {
    pub async fn authorize(&self) -> String {
        let (auth_url, _) = self // TODO: use csrf_token
            .oauth2
            .authorize_url(CsrfToken::new_random)
            .add_extra_param("audience", self.config.audience.clone())
            .add_scopes([
                Scope::new("openid".to_string()),
                Scope::new("profile".to_string()),
            ])
            .url();
        auth_url.to_string()
    }

    pub async fn exchange_code(&self, code: &str) -> Result<String> {
        let code = AuthorizationCode::new(code.to_string());

        let token_result = self
            .oauth2
            .exchange_code(code)
            .request_async(async_http_client)
            .await
            .expect("Failed to exchange code"); // FIXME: handle error

        Ok(token_result.access_token().secret().to_owned())
    }

    pub async fn validate(&self, token: &str) -> Result<TokenClaims> {
        let kid = self.get_kid(token)?;
        let decoding_keys_guard = self.jwk_decoding_keys.read().await;
        let decoding_key = decoding_keys_guard
            .get(&kid)
            .ok_or(auth::Error::UnknownKid)?;

        decode::<TokenClaims>(token, decoding_key, &self.jwt_validator)
            .map(|data| data.claims)
            .map_err(|e| auth::Error::Forbidden(e.to_string()))
    }

    pub async fn get_user_info(&self, token: &str) -> Result<UserInfo> {
        let user_info = self
            .http
            .get(&self.config.userinfo_url)
            .bearer_auth(token)
            .send()
            .await?
            .json::<UserInfo>()
            .await?;

        Ok(user_info)
    }
}

impl AuthService {
    fn get_kid(&self, token: &str) -> Result<String> {
        let jwt_header =
            decode_header(token).map_err(|e| auth::Error::TokenMalformed(e.to_string()))?;

        jwt_header
            .kid
            .map(|kid| kid.to_string())
            .ok_or(auth::Error::UnknownKid)
    }
}

async fn fetch_jwk_decoding_keys(
    config: &idp::Config,
    http: &reqwest::Client,
) -> Result<HashMap<String, DecodingKey>> {
    let jwk_response = http.get(&config.jwks_url).send().await?;
    let jwk_json = jwk_response.json().await?;
    let jwk_set: JwkSet = serde_json::from_value(jwk_json)?;

    let mut jwk_decoding_keys = HashMap::new();

    for jwk in jwk_set.keys.iter() {
        if let Some(kid) = jwk.clone().common.key_id {
            let key =
                DecodingKey::from_jwk(jwk).map_err(|e| auth::Error::Unexpected(e.to_string()))?;
            jwk_decoding_keys.insert(kid, key);
        }
    }

    Ok(jwk_decoding_keys)
}
