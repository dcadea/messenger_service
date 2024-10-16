use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AccessToken, AuthorizationCode, CsrfToken, Scope, TokenResponse};
use tokio::sync::RwLock;
use tokio::time::sleep;

use redis::AsyncCommands;

use super::TokenClaims;

use crate::integration::idp;
use crate::integration::{self, cache};
use crate::user;
use crate::user::model::UserInfo;

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);
// TODO: use ttl from token response
const TOKEN_TTL: Duration = Duration::from_secs(36000);
// TODO: use ttl from application config
const EXCHANGE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct AuthService {
    config: Arc<idp::Config>,
    http: Arc<reqwest::Client>,
    oauth2: Arc<BasicClient>,
    redis_con: redis::aio::ConnectionManager,
    jwt_validator: Arc<Validation>,
    jwk_decoding_keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
}

impl AuthService {
    pub fn try_new(
        config: &idp::Config,
        redis_con: redis::aio::ConnectionManager,
    ) -> super::Result<Self> {
        let mut jwt_validator = Validation::new(jsonwebtoken::Algorithm::RS256);
        jwt_validator.set_required_spec_claims(&config.required_claims);
        jwt_validator.set_issuer(&[&config.issuer]);
        jwt_validator.set_audience(&[&config.audience]);

        let jwk_decoding_keys = Arc::new(RwLock::new(HashMap::new()));
        let service = Self {
            config: Arc::new(config.to_owned()),
            http: Arc::new(integration::init_http_client()?),
            oauth2: Arc::new(integration::idp::init(config)),
            redis_con,
            jwt_validator: Arc::new(jwt_validator),
            jwk_decoding_keys: jwk_decoding_keys.clone(),
        };

        let http = integration::init_http_client()?;
        let config_clone = config.clone();
        tokio::spawn(async move {
            loop {
                match fetch_jwk_decoding_keys(&config_clone, &http).await {
                    Ok(keys) => *jwk_decoding_keys.write().await = keys,
                    Err(e) => eprintln!("Failed to update JWK decoding keys: {:?}", e),
                }
                sleep(ONE_DAY).await;
            }
        });

        Ok(service)
    }
}

impl AuthService {
    pub async fn authorize(&self) -> super::Result<String> {
        let (auth_url, csrf) = self
            .oauth2
            .authorize_url(CsrfToken::new_random)
            .add_extra_param("audience", self.config.audience.clone())
            .add_scopes([
                Scope::new("openid".to_string()),
                Scope::new("profile".to_string()),
            ])
            .url();

        self.cache_csrf(csrf.secret()).await?;

        Ok(auth_url.to_string())
    }

    pub async fn exchange_code(&self, code: &str, csrf: &str) -> super::Result<AccessToken> {
        self.validate_state(csrf).await?;

        let code = AuthorizationCode::new(code.to_string());

        let token_result = self
            .oauth2
            .exchange_code(code)
            .request_async(async_http_client)
            .await
            .map_err(|e| super::Error::Unexpected(e.to_string()))?;

        Ok(token_result.access_token().to_owned())
    }

    pub async fn validate(&self, token: &str) -> super::Result<user::Sub> {
        let kid = self.get_kid(token)?;
        let decoding_keys_guard = self.jwk_decoding_keys.read().await;
        let decoding_key = decoding_keys_guard
            .get(&kid)
            .ok_or(super::Error::UnknownKid)?;

        decode::<TokenClaims>(token, decoding_key, &self.jwt_validator)
            .map(|data| data.claims)
            .map(|claims| claims.sub)
            .map_err(|e| super::Error::Forbidden(e.to_string()))
    }

    pub async fn get_user_info(&self, token: &str) -> super::Result<UserInfo> {
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
    fn get_kid(&self, token: &str) -> super::Result<String> {
        let jwt_header =
            decode_header(token).map_err(|e| super::Error::TokenMalformed(e.to_string()))?;

        jwt_header
            .kid
            .map(|kid| kid.to_string())
            .ok_or(super::Error::UnknownKid)
    }

    pub async fn cache_token(&self, sid: &uuid::Uuid, token: &str) -> super::Result<()> {
        let mut con = self.redis_con.clone();
        let cache_key = cache::Key::Session(sid.to_string());
        let _: () = con.set_ex(cache_key, token, TOKEN_TTL.as_secs()).await?;
        Ok(())
    }

    pub async fn invalidate_token(&self, sid: &str) -> super::Result<()> {
        let mut con = self.redis_con.clone();
        let sid = cache::Key::Session(sid.to_string());
        let _: () = con.del(sid).await?;
        Ok(())
    }

    pub async fn find_token(&self, sid: &str) -> Option<String> {
        let mut con = self.redis_con.clone();
        let sid = cache::Key::Session(sid.to_string());
        let token: Option<String> = con.get(sid).await.ok();
        token
    }

    async fn cache_csrf(&self, csrf: &str) -> super::Result<()> {
        let mut con = self.redis_con.clone();
        let cache_key = cache::Key::Csrf(csrf.to_string());
        let _: () = con.set_ex(cache_key, csrf, EXCHANGE_TTL.as_secs()).await?;
        Ok(())
    }

    async fn validate_state(&self, csrf: &str) -> super::Result<()> {
        let mut con = self.redis_con.clone();
        let cache_key = cache::Key::Csrf(csrf.to_string());
        let csrf: Option<String> = con.get_del(cache_key).await?;
        csrf.map(|_| ()).ok_or(super::Error::InvalidState)
    }
}

async fn fetch_jwk_decoding_keys(
    config: &idp::Config,
    http: &reqwest::Client,
) -> super::Result<HashMap<String, DecodingKey>> {
    let jwk_response = http.get(&config.jwks_url).send().await?;
    let jwk_json = jwk_response.json().await?;
    let jwk_set: JwkSet = serde_json::from_value(jwk_json)?;

    let mut jwk_decoding_keys = HashMap::new();

    for jwk in jwk_set.keys.iter() {
        if let Some(kid) = jwk.clone().common.key_id {
            let key =
                DecodingKey::from_jwk(jwk).map_err(|e| super::Error::Unexpected(e.to_string()))?;
            jwk_decoding_keys.insert(kid, key);
        }
    }

    Ok(jwk_decoding_keys)
}
