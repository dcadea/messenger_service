use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use axum::extract::FromRef;

use jsonwebtoken;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::DecodingKey;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::chat::repository::ChatRepository;
use crate::chat::service::ChatService;
use crate::error::ApiError;
use crate::integration;
use crate::message::repository::MessageRepository;
use crate::message::service::MessageService;
use crate::result::Result;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;

#[derive(Clone)]
pub(crate) struct AppState {
    pub config: integration::Config,
    pub auth_state: AuthState,

    pub message_service: Arc<MessageService>,
    pub chat_service: Arc<ChatService>,
    pub user_service: Arc<UserService>,

    pub http: Arc<reqwest::Client>,
}

impl AppState {
    pub async fn init() -> Result<Self> {
        let config = integration::Config::default();
        let auth_state = AuthState::init(&config).await?;
        let database = integration::init_mongodb(&config).await?;
        let _ = integration::init_redis(&config)?;
        let rabbitmq_con = integration::init_rabbitmq(&config).await?;

        Ok(Self {
            config,
            auth_state,
            message_service: MessageService::new(MessageRepository::new(&database), rabbitmq_con),
            chat_service: ChatService::new(ChatRepository::new(&database)),
            user_service: UserService::new(UserRepository::new(&database)),
            http: integration::init_http_client()?,
        })
    }
}

#[derive(Clone)]
pub(crate) struct AuthState {
    pub jwt_validator: Arc<jsonwebtoken::Validation>,
    pub jwk_decoding_keys: Arc<Mutex<HashMap<String, DecodingKey>>>,
}

impl AuthState {
    async fn init(config: &integration::Config) -> Result<Self> {
        let mut jwk_validator = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        jwk_validator.set_required_spec_claims(&vec!["iss", "sub", "aud", "exp", "privileges"]);
        jwk_validator.set_issuer(&[&config.issuer]);
        jwk_validator.set_audience(&config.audience);

        let jwk_decoding_keys = Arc::new(Mutex::new(HashMap::new()));
        let state = Self {
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

        Ok(state)
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

impl FromRef<AppState> for integration::Config {
    fn from_ref(app_state: &AppState) -> integration::Config {
        app_state.config.clone()
    }
}

impl FromRef<AppState> for AuthState {
    fn from_ref(app_state: &AppState) -> AuthState {
        app_state.auth_state.clone()
    }
}

impl FromRef<AppState> for Arc<MessageService> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.message_service.clone()
    }
}

impl FromRef<AppState> for Arc<ChatService> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.chat_service.clone()
    }
}

impl FromRef<AppState> for Arc<UserService> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.user_service.clone()
    }
}
