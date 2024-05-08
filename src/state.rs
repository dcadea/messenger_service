use axum::extract::FromRef;
use std::sync::Arc;

use crate::auth::service::AuthService;

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
    pub config: integration::Config,
    pub auth_service: AuthService,

    pub message_service: Arc<MessageService>,
    pub chat_service: Arc<ChatService>,
    pub user_service: Arc<UserService>,
}

impl AppState {
    pub async fn init() -> Result<Self> {
        let config = integration::Config::default();
        let auth_service = AuthService::try_new(&config)?;
        let database = integration::init_mongodb(&config).await?;
        let _ = integration::init_redis(&config)?;
        let rabbitmq_con = integration::init_rabbitmq(&config).await?;

        Ok(Self {
            config,
            auth_service,
            message_service: MessageService::new(MessageRepository::new(&database), rabbitmq_con),
            chat_service: ChatService::new(ChatRepository::new(&database)),
            user_service: UserService::new(UserRepository::new(&database)),
        })
    }
}

impl FromRef<AppState> for integration::Config {
    fn from_ref(app_state: &AppState) -> integration::Config {
        app_state.config.clone()
    }
}

impl FromRef<AppState> for AuthService {
    fn from_ref(app_state: &AppState) -> AuthService {
        app_state.auth_service.clone()
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
