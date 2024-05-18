use axum::extract::FromRef;

use super::auth::service::AuthService;
use super::chat::repository::ChatRepository;
use super::chat::service::ChatService;
use super::event::service::EventService;
use super::integration;
use super::message::repository::MessageRepository;
use super::message::service::MessageService;
use super::result::Result;
use super::user::repository::UserRepository;
use super::user::service::UserService;

#[derive(Clone)]
pub struct AppState {
    pub config: integration::Config,

    pub auth_service: AuthService,
    pub user_service: UserService,
    pub chat_service: ChatService,
    pub message_service: MessageService,
    pub event_service: EventService,
}

impl AppState {
    pub async fn init() -> Result<Self> {
        let config = integration::Config::default();
        let auth_service = AuthService::try_new(&config)?;
        let database = integration::init_mongodb(&config).await?;
        let _ = integration::init_redis(&config)?;
        let rabbitmq_con = integration::init_rabbitmq(&config).await?;

        let user_service = UserService::new(UserRepository::new(&database));
        let chat_service = ChatService::new(ChatRepository::new(&database));
        let message_service = MessageService::new(MessageRepository::new(&database));
        let event_service =
            EventService::new(rabbitmq_con, message_service.clone(), auth_service.clone());

        Ok(Self {
            config,
            auth_service,
            user_service,
            chat_service,
            message_service,
            event_service,
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

impl FromRef<AppState> for UserService {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.user_service.clone()
    }
}

impl FromRef<AppState> for ChatService {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.chat_service.clone()
    }
}

impl FromRef<AppState> for MessageService {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.message_service.clone()
    }
}

impl FromRef<AppState> for EventService {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.event_service.clone()
    }
}
