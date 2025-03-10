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
use uuid::Uuid;

use super::TokenClaims;

use crate::integration::cache;
use crate::integration::idp;
use crate::integration::{self};
use crate::user;
use crate::user::model::UserInfo;

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone)]
pub struct AuthService {
    cfg: Arc<idp::Config>,
    http: Arc<reqwest::Client>,
    oauth2: Arc<BasicClient>,
    redis: cache::Redis,
    jwt_validator: Arc<Validation>,
    jwk_decoding_keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
}

impl AuthService {
    pub fn try_new(cfg: &idp::Config, redis: cache::Redis) -> super::Result<Self> {
        let mut jwt_validator = Validation::new(jsonwebtoken::Algorithm::RS256);
        jwt_validator.set_required_spec_claims(&cfg.required_claims);
        jwt_validator.set_issuer(&[&cfg.issuer]);
        jwt_validator.set_audience(&[&cfg.audience]);

        let jwk_decoding_keys = Arc::new(RwLock::new(HashMap::new()));
        let service = Self {
            cfg: Arc::new(cfg.to_owned()),
            http: Arc::new(integration::init_http_client()),
            oauth2: Arc::new(cfg.init_client()),
            redis,
            jwt_validator: Arc::new(jwt_validator),
            jwk_decoding_keys: jwk_decoding_keys.clone(),
        };

        // FIXME: this occupies a resource for too long
        let jwks_url = cfg.jwks_url.clone();
        tokio::spawn(async move {
            let http = integration::init_http_client();
            loop {
                match fetch_jwk_decoding_keys(&jwks_url, &http).await {
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
            .add_extra_param("audience", self.cfg.audience.clone())
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
            Ok(r) => {
                let access_token = r.access_token().to_owned();
                let expires_in = r.expires_in().unwrap_or(self.cfg.token_ttl);

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
            .get(&self.cfg.userinfo_url)
            .bearer_auth(token)
            .send()
            .await?;

        Ok(response.json::<UserInfo>().await?)
    }
}

impl AuthService {
    pub async fn cache_token(&self, sid: &Uuid, token: &str, ttl: &Duration) {
        self.redis.set(cache::Key::Session(*sid), token).await;
        self.redis
            .expire_after(cache::Key::Session(*sid), ttl.as_secs())
            .await;
    }

    pub async fn invalidate_token(&self, sid: &str) -> super::Result<()> {
        let sid = Uuid::parse_str(sid)?;
        let token = self.redis.get_del(cache::Key::Session(sid)).await;

        if let Some(token) = token {
            self.oauth2
                .revoke_token(StandardRevocableToken::AccessToken(AccessToken::new(token)))?;
        }

        Ok(())
    }

    pub async fn find_token(&self, sid: &str) -> Option<String> {
        match Uuid::parse_str(sid) {
            Ok(sid) => self.redis.get::<String>(cache::Key::Session(sid)).await,
            Err(_) => {
                warn!("Could not find token for sid: {sid}");
                None
            }
        }
    }

    async fn cache_csrf(&self, csrf: &str) {
        let csrf_key = cache::Key::Csrf(csrf.into());
        self.redis.set_ex(csrf_key, csrf).await
    }

    async fn validate_state(&self, csrf: &str) -> super::Result<()> {
        let csrf_key = cache::Key::Csrf(csrf.into());
        let cached_csrf = self.redis.get_del::<String>(csrf_key).await;

        cached_csrf
            .filter(|cc| cc == csrf)
            .map(|_| ())
            .ok_or(super::Error::InvalidState)
    }
}

async fn fetch_jwk_decoding_keys(
    jwks_url: &str,
    http: &reqwest::Client,
) -> super::Result<HashMap<String, DecodingKey>> {
    let jwk_response = http.get(jwks_url).send().await?;
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
