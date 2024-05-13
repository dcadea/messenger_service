use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::auth::model::TokenClaims;
use crate::auth::model::UserInfo;
use crate::error::ApiError;
use crate::integration;
use crate::result::Result;

#[derive(Clone)]
pub struct AuthService {
    config: Arc<integration::Config>,
    http: Arc<reqwest::Client>,
    jwt_validator: Arc<Validation>,
    jwk_decoding_keys: Arc<Mutex<HashMap<String, DecodingKey>>>,
}

impl AuthService {
    pub fn try_new(config: &integration::Config) -> Result<Self> {
        let mut jwk_validator = Validation::new(jsonwebtoken::Algorithm::RS256);
        jwk_validator.set_required_spec_claims(&vec!["iss", "sub", "aud", "exp", "privileges"]);
        jwk_validator.set_issuer(&[&config.issuer]);
        jwk_validator.set_audience(&config.audience);

        let jwk_decoding_keys = Arc::new(Mutex::new(HashMap::new()));
        let service = Self {
            config: Arc::new(config.clone()),
            http: Arc::new(integration::init_http_client()?),
            jwt_validator: Arc::new(jwk_validator),
            jwk_decoding_keys: jwk_decoding_keys.clone(),
        };

        let config_clone = config.clone();
        tokio::spawn(async move {
            loop {
                match fetch_jwk_decoding_keys(&config_clone).await {
                    Ok(keys) => *jwk_decoding_keys.lock().await = keys,
                    Err(e) => eprintln!("Failed to update JWK decoding keys: {:?}", e),
                }
                sleep(Duration::from_secs(24 * 60 * 60)).await;
            }
        });

        Ok(service)
    }
}

impl AuthService {
    pub async fn validate(&self, token: &str) -> Result<TokenClaims> {
        let kid = self.get_kid(token)?;
        let decoding_keys_guard = self.jwk_decoding_keys.lock().await;
        let decoding_key = decoding_keys_guard
            .get(&kid)
            .ok_or(ApiError::Forbidden("Unknown kid".to_owned()))?;

        decode::<TokenClaims>(token, &decoding_key, &self.jwt_validator)
            .map(|data| data.claims)
            .map_err(|e| ApiError::Forbidden(e.to_string()))
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

    fn get_kid(&self, token: &str) -> Result<String> {
        let jwt_header =
            decode_header(token).map_err(|e| ApiError::TokenMalformed(e.to_string()))?;
        let kid = jwt_header
            .kid
            .as_ref()
            .ok_or(ApiError::Forbidden("Missing kid".to_owned()))?;
        Ok(kid.to_string())
    }
}

async fn fetch_jwk_decoding_keys(
    config: &integration::Config,
) -> Result<HashMap<String, DecodingKey>> {
    let http = integration::init_http_client()?;
    let jwk_response = http.get(config.jwks_url.clone()).send().await?;
    let jwk_json = jwk_response.json().await?;
    let jwk_set: JwkSet = serde_json::from_value(jwk_json)?;
    let mut jwk_decoding_keys = HashMap::new();
    for jwk in jwk_set.keys.iter() {
        if let Some(kid) = jwk.clone().common.key_id {
            let key = DecodingKey::from_jwk(jwk)
                .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
            jwk_decoding_keys.insert(kid, key);
        }
    }

    Ok(jwk_decoding_keys)
}
