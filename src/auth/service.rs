use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use log::{debug, error, warn};
use messenger_service::Raw;
use oauth2::{AccessToken, CsrfToken, Scope, StandardRevocableToken, TokenResponse};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{Code, Csrf, TokenClaims};

use crate::integration::cache;
use crate::integration::idp;
use crate::integration::{self};
use crate::user;
use crate::user::model::UserInfo;

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY_DELAY: Duration = Duration::from_secs(15);

#[async_trait]
pub trait AuthService {
    async fn authorize(&self) -> String;

    async fn exchange_code(&self, code: Code, csrf: Csrf)
    -> super::Result<(AccessToken, Duration)>;

    async fn validate(&self, token: &str) -> super::Result<user::Sub>;

    async fn get_user_info(&self, token: &str) -> super::Result<UserInfo>;

    async fn cache_token(&self, sid: &Uuid, token: &str, ttl: &Duration);

    async fn invalidate_token(&self, sid: &str) -> super::Result<()>;

    async fn find_token(&self, sid: &str) -> Option<String>;
}

#[derive(Clone)]
pub struct AuthServiceImpl {
    cfg: Arc<idp::Config>,
    http: Arc<reqwest::Client>,
    oauth2: Arc<idp::OAuth2Client>,
    redis: cache::Redis,
    jwt_validator: Arc<Validation>,
    jwk_decoding_keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
}

impl AuthServiceImpl {
    pub fn try_new(cfg: &idp::Config, redis: cache::Redis) -> Self {
        let jwt_validator = {
            let mut v = Validation::new(jsonwebtoken::Algorithm::RS256);
            v.set_required_spec_claims(cfg.required_claims());
            v.set_issuer(&[cfg.issuer()]);
            v.set_audience(&[cfg.audience()]);
            v
        };

        let jwk_decoding_keys = Arc::new(RwLock::new(HashMap::new()));
        let service = Self {
            cfg: Arc::new(cfg.to_owned()),
            http: Arc::new(integration::init_http_client()),
            oauth2: Arc::new(cfg.init_client()),
            redis,
            jwt_validator: Arc::new(jwt_validator),
            jwk_decoding_keys: jwk_decoding_keys.clone(),
        };

        let jwks_url = cfg.jwks_url().to_string();
        tokio::spawn(async move {
            let http = integration::init_http_client();
            loop {
                match fetch_jwk_decoding_keys(&jwks_url, &http).await {
                    Ok(keys) => *jwk_decoding_keys.write().await = keys,
                    Err(e) => {
                        error!("Failed to fetch JWKs: {e:?}");
                        debug!(
                            "Retrying to fetch JWKs in {} seconds",
                            RETRY_DELAY.as_secs()
                        );
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                }
                tokio::time::sleep(ONE_DAY).await;
            }
        });

        service
    }
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn authorize(&self) -> String {
        let (auth_url, csrf) = self
            .oauth2
            .authorize_url(CsrfToken::new_random)
            .add_extra_param("audience", self.cfg.audience())
            .add_scopes([
                Scope::new("openid".to_string()),
                Scope::new("profile".to_string()),
                Scope::new("email".to_string()),
            ])
            .url();

        self.cache_csrf(csrf).await;

        auth_url.to_string()
    }

    async fn exchange_code(
        &self,
        code: Code,
        csrf: Csrf,
    ) -> super::Result<(AccessToken, Duration)> {
        self.validate_state(csrf).await?;

        debug!("Exchanging code '{code:?}' for token");

        let token_result = self
            .oauth2
            .exchange_code(code.into())
            .request_async(&*self.http)
            .await;

        match token_result {
            Ok(r) => {
                let access_token = r.access_token().to_owned();
                let expires_in = r.expires_in().unwrap_or(self.cfg.token_ttl());

                Ok((access_token, expires_in))
            }
            Err(e) => Err(super::Error::Unexpected(e.to_string())),
        }
    }

    async fn validate(&self, token: &str) -> super::Result<user::Sub> {
        let jwt_header = decode_header(token).map_err(|e| {
            warn!("Failed to decode JWT header: {e:?}");
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
                error!("Failed to decode token claims: {e:?}");
                super::Error::Forbidden
            })
    }

    async fn get_user_info(&self, token: &str) -> super::Result<UserInfo> {
        let response = self
            .http
            .get(self.cfg.userinfo_url())
            .bearer_auth(token)
            .send()
            .await?;

        let u = response.json::<UserInfo>().await?;
        debug!("User info retrieved: {u:?}");
        Ok(u)
    }

    async fn cache_token(&self, sid: &Uuid, token: &str, ttl: &Duration) {
        debug!("Caching token for sid '{sid}'");

        self.redis
            .set_ex_explicit(cache::Key::Session(sid), token, ttl)
            .await;
    }

    async fn invalidate_token(&self, sid: &str) -> super::Result<()> {
        debug!("Invalidating token for sid '{sid}'");

        let sid = Uuid::parse_str(sid)?;
        let token = self.redis.get_del(cache::Key::Session(&sid)).await;

        if let Some(token) = token {
            self.oauth2
                .revoke_token(StandardRevocableToken::AccessToken(AccessToken::new(token)))?;
            debug!("Token for sid '{sid}' revoked");
        }

        Ok(())
    }

    async fn find_token(&self, sid: &str) -> Option<String> {
        if let Ok(sid) = Uuid::parse_str(sid) {
            self.redis.get::<String>(cache::Key::Session(&sid)).await
        } else {
            error!("Could not find token for sid '{sid}'");
            None
        }
    }
}

impl AuthServiceImpl {
    async fn cache_csrf(&self, csrf: impl Into<Csrf>) {
        let csrf = csrf.into();
        debug!("Caching csrf '{csrf:?}'");

        let csrf_key = cache::Key::Csrf(&csrf);
        self.redis.set_ex(csrf_key, csrf.raw()).await;
    }

    async fn validate_state(&self, csrf: Csrf) -> super::Result<()> {
        debug!("Validating state for csrf '{csrf:?}'");
        let csrf_key = cache::Key::Csrf(&csrf);
        let cached_csrf = self.redis.get_del::<Csrf>(csrf_key).await;

        if cached_csrf.is_some_and(|cc| cc.eq(&csrf)) {
            return Ok(());
        }

        Err(super::Error::InvalidState)
    }
}

async fn fetch_jwk_decoding_keys(
    jwks_url: &str,
    http: &reqwest::Client,
) -> super::Result<HashMap<String, DecodingKey>> {
    let jwk_response = http.get(jwks_url).send().await?;
    let jwk_set: JwkSet = jwk_response.json().await?;

    let jwk_decoding_keys = {
        let mut keys = HashMap::new();

        for jwk in &jwk_set.keys {
            if let Some(kid) = jwk.clone().common.key_id {
                let key = DecodingKey::from_jwk(jwk)?;

                debug!("Fetched jwk with id '{kid}'");
                keys.insert(kid, key);
            }
        }

        keys
    };

    Ok(jwk_decoding_keys)
}
