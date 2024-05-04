use jsonwebtoken;
use jsonwebtoken::jwk::JwkSet;
use std::sync::Arc;

use crate::chat::repository::ChatRepository;
use crate::chat::service::ChatService;
use crate::integration;
use crate::message::repository::MessageRepository;
use crate::message::service::MessageService;
use crate::result::Result;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;

#[derive(Clone)]
pub(crate) struct AppState {
    pub message_service: Arc<MessageService>,
    pub chat_service: Arc<ChatService>,
    pub user_service: Arc<UserService>,

    // uncomment when needed
    // pub http: Arc<reqwest::Client>,
    pub jwk_set: Arc<JwkSet>,
    pub jwt_validator: Arc<jsonwebtoken::Validation>,
}

impl AppState {
    pub async fn init(config: &integration::Config) -> Result<Self> {
        let database = integration::init_mongodb(config).await?;
        let _ = integration::init_redis(config)?;
        let rabbitmq_con = integration::init_rabbitmq(config).await?;

        // TODO: find a better place for this
        let http = integration::init_http_client()?;
        let jwk_response = http.get(config.jwks_url.clone()).send().await?;
        let jwk_json = jwk_response.json().await?;
        let jwk_set: JwkSet = serde_json::from_value(jwk_json)?;

        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&config.audience);

        Ok(Self {
            message_service: MessageService::new(MessageRepository::new(&database), rabbitmq_con),
            chat_service: ChatService::new(ChatRepository::new(&database)),
            user_service: UserService::new(UserRepository::new(&database)),
            jwk_set: Arc::new(jwk_set),
            jwt_validator: Arc::new(validation),
        })
    }
}

// TODO: investigate
// impl FromRef<AppState> for MessageService {
//     fn from_ref(state: &AppState) -> Arc<Self> {
//         state.message_service.clone()
//     }
// }
