use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::jwk::JwkSet;
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
    pub config: Arc<integration::Config>,

    pub message_service: Arc<MessageService>,
    pub chat_service: Arc<ChatService>,
    pub user_service: Arc<UserService>,

    pub http: Arc<reqwest::Client>,
}

impl AppState {
    pub async fn init() -> Result<Self> {
        let config = integration::Config::default();

        let database = integration::init_mongodb(&config).await?;
        let _ = integration::init_redis(&config)?;
        let rabbitmq_con = integration::init_rabbitmq(&config).await?;

        Ok(Self {
            config: Arc::new(config),
            message_service: MessageService::new(MessageRepository::new(&database), rabbitmq_con),
            chat_service: ChatService::new(ChatRepository::new(&database)),
            user_service: UserService::new(UserRepository::new(&database)),
            http: integration::init_http_client()?,
        })
    }
}

// TODO: investigate
// impl FromRef<AppState> for MessageService {
//     fn from_ref(state: &AppState) -> Arc<Self> {
//         state.message_service.clone()
//     }
// }

#[derive(Clone)]
pub(crate) struct AuthState {
    pub jwt_validator: Arc<jsonwebtoken::Validation>,
    pub jwk_decoding_keys: Arc<Mutex<HashMap<String, DecodingKey>>>,
}

impl AuthState {
    pub async fn init() -> Result<Self> {
        let config = integration::Config::default(); // TODO: refactor to use common config

        let mut jwk_validator = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
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
                    Err(e) => eprintln!("Failed to update JWK decoding keys: {:?}", e)
                }
                sleep(Duration::from_secs(24 * 60 * 60)).await;
            }
        });

        Ok(state)
    }
}

async fn fetch_jwk_decoding_keys(
    config: &integration::Config
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