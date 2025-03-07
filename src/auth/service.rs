use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use log::{error, warn};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AccessToken, AuthorizationCode, CsrfToken, Scope, StandardRevocableToken, TokenResponse,
};
use tokio::sync::RwLock;

use super::TokenClaims;

use crate::integration;
use crate::integration::cache;
use crate::integration::idp;
use crate::user;
use crate::user::model::UserInfo;

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone)]
pub struct AuthService {
    config: Arc<idp::Config>,
    http: Arc<reqwest::Client>,
    oauth2: Arc<BasicClient>,
    redis: integration::cache::Redis,
    jwt_validator: Arc<Validation>,
    jwk_decoding_keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
}

impl AuthService {
    pub fn try_new(config: &idp::Config, redis: integration::cache::Redis) -> super::Result<Self> {
        let mut jwt_validator = Validation::new(jsonwebtoken::Algorithm::RS256);
        jwt_validator.set_required_spec_claims(&config.required_claims);
        jwt_validator.set_issuer(&[&config.issuer]);
        jwt_validator.set_audience(&[&config.audience]);

        let jwk_decoding_keys = Arc::new(RwLock::new(HashMap::new()));
        let service = Self {
            config: Arc::new(config.to_owned()),
            http: Arc::new(integration::init_http_client()),
            oauth2: Arc::new(integration::idp::init(config)),
            redis,
            jwt_validator: Arc::new(jwt_validator),
            jwk_decoding_keys: jwk_decoding_keys.clone(),
        };

        let config_clone = config.clone();
        // FIXME: this occupies a resource for too long
        tokio::spawn(async move {
            let http = integration::init_http_client();
            loop {
                match fetch_jwk_decoding_keys(&config_clone, &http).await {
                    Ok(keys) => *jwk_decoding_keys.write().await = keys,
                    Err(e) => error!("Failed to update JWK decoding keys: {e:?}"),
                }
                tokio::time::sleep(ONE_DAY).await;
            }
        });

        Ok(service)
    }
}

impl AuthService {
    pub async fn authorize(&self) -> String {
        let (auth_url, csrf) = self
            .oauth2
            .authorize_url(CsrfToken::new_random)
            .add_extra_param("audience", self.config.audience.clone())
            .add_scopes([
                Scope::new("openid".to_string()),
                Scope::new("profile".to_string()),
                Scope::new("email".to_string()),
            ])
            .url();

        self.cache_csrf(csrf.secret()).await;

        auth_url.to_string()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        csrf: &str,
    ) -> super::Result<(AccessToken, Duration)> {
        self.validate_state(csrf).await?;

        let auth_code = AuthorizationCode::new(code.to_string());

        let token_result = self
            .oauth2
            .exchange_code(auth_code)
            .request_async(async_http_client)
            .await;

        match token_result {
            Ok(response) => {
                let access_token = response.access_token().to_owned();
                let expires_in = response.expires_in().unwrap_or(self.config.token_ttl);

                Ok((access_token, expires_in))
            }
            Err(e) => Err(super::Error::_Unexpected(e.to_string())),
        }
    }

    pub async fn validate(&self, token: &str) -> super::Result<user::Sub> {
        let jwt_header = decode_header(token).map_err(|e| {
            warn!("{e:?}");
            super::Error::TokenMalformed
        })?;

        let kid = jwt_header.kid.ok_or(super::Error::UnknownKid)?;
        let decoding_keys_guard = self.jwk_decoding_keys.read().await;
        let decoding_key = decoding_keys_guard
            .get(&kid)
            .ok_or(super::Error::UnknownKid)?;

        decode::<TokenClaims>(token, decoding_key, &self.jwt_validator)
            .map(|data| data.claims)
            .map(|claims| claims.sub)
            .map_err(|e| {
                warn!("{e:?}");
                super::Error::Forbidden
            })
    }

    pub async fn get_user_info(&self, token: &str) -> super::Result<UserInfo> {
        let response = self
            .http
            .get(&self.config.userinfo_url)
            .bearer_auth(token)
            .send()
            .await?;

        let user_info: UserInfo = response.json().await?;

        Ok(user_info)
    }
}

impl AuthService {
    pub async fn cache_token(&self, sid: &uuid::Uuid, token: &str, ttl: &Duration) {
        self.redis.set(cache::Key::Session(*sid), token).await;
        self.redis
            .expire_after(cache::Key::Session(*sid), ttl.as_secs())
            .await;
    }

    pub async fn invalidate_token(&self, sid: &str) -> super::Result<()> {
        let sid = uuid::Uuid::parse_str(sid)?;
        let token = self.redis.get_del(cache::Key::Session(sid)).await;

        if let Some(token) = token {
            self.oauth2
                .revoke_token(StandardRevocableToken::AccessToken(AccessToken::new(token)))?;
        }

        Ok(())
    }

    pub async fn find_token(&self, sid: &str) -> Option<String> {
        match uuid::Uuid::parse_str(sid) {
            Ok(sid) => self.redis.get::<String>(cache::Key::Session(sid)).await,
            Err(_) => {
                warn!("Could not find token for sid: {sid}");
                None
            }
        }
    }

    async fn cache_csrf(&self, csrf: &str) {
        let cache_key = cache::Key::Csrf(csrf.to_string());
        self.redis.set_ex(cache_key, csrf).await
    }

    async fn validate_state(&self, csrf: &str) -> super::Result<()> {
        let cache_key = cache::Key::Csrf(csrf.to_string());
        let cached_csrf = self.redis.get_del::<String>(cache_key).await;

        cached_csrf
            .filter(|cc| cc == csrf)
            .map(|_| ())
            .ok_or(super::Error::InvalidState)
    }
}

async fn fetch_jwk_decoding_keys(
    config: &idp::Config,
    http: &reqwest::Client,
) -> super::Result<HashMap<String, DecodingKey>> {
    let jwk_response = http.get(&config.jwks_url).send().await?;
    let jwk_set: JwkSet = jwk_response.json().await?;

    let mut jwk_decoding_keys = HashMap::new();

    for jwk in jwk_set.keys.iter() {
        if let Some(kid) = jwk.clone().common.key_id {
            let key = DecodingKey::from_jwk(jwk)?;
            jwk_decoding_keys.insert(kid, key);
        }
    }

    Ok(jwk_decoding_keys)
}
