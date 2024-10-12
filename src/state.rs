use axum::extract::FromRef;

use super::auth::service::AuthService;
use super::chat::repository::ChatRepository;
use super::chat::service::ChatService;
use super::event::service::EventService;
use super::integration;
use super::message::repository::MessageRepository;
use super::message::service::MessageService;
use super::user::repository::UserRepository;
use super::user::service::UserService;

#[derive(Clone)]
pub(crate) struct AppState {
    pub config: integration::Config,

    pub auth_service: AuthService,
    pub user_service: UserService,
    pub chat_service: ChatService,
    pub message_service: MessageService,
    pub event_service: EventService,
}

impl AppState {
    pub async fn init(config: integration::Config) -> crate::Result<Self> {
        let database = integration::db::init(&config.mongo).await?;
        let redis_con = integration::cache::init(&config.redis).await?;
        let amqp_con = integration::amqp::init(&config.amqp).await?;

        let auth_service = AuthService::try_new(&config.idp)?;
        let user_service = UserService::new(redis_con.clone(), UserRepository::new(&database));
        let chat_service = ChatService::new(
            ChatRepository::new(&database),
            user_service.clone(),
            redis_con.clone(),
        );
        let message_service = MessageService::new(MessageRepository::new(&database));
        let event_service = EventService::new(
            amqp_con,
            auth_service.clone(),
            chat_service.clone(),
            message_service.clone(),
            user_service.clone(),
        );

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
