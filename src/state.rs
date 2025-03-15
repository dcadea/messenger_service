use axum::extract::FromRef;

use crate::chat::service::ChatValidator;

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
pub struct State {
    pub cfg: integration::Config,

    pub auth_service: AuthService,
    pub user_service: UserService,
    pub chat_validator: ChatValidator,
    pub chat_service: ChatService,
    pub message_service: MessageService,
    pub event_service: EventService,
}

impl State {
    pub async fn init(cfg: integration::Config) -> crate::Result<Self> {
        let db = cfg.mongo.connect();
        let redis = cfg.redis.connect().await;
        let pubsub = cfg.pubsub.connect().await;

        let auth_service = AuthService::try_new(&cfg.idp, redis.clone())?;
        let event_service = EventService::new(pubsub);
        let user_service = UserService::new(
            UserRepository::new(&db),
            event_service.clone(),
            redis.clone(),
        );

        let chat_repo = ChatRepository::new(&db);
        let message_repo = MessageRepository::new(&db);

        let chat_validator = ChatValidator::new(chat_repo.clone(), redis.clone());
        let chat_service = ChatService::new(
            chat_repo.clone(),
            chat_validator.clone(),
            message_repo.clone(),
            user_service.clone(),
            event_service.clone(),
            redis.clone(),
        );

        let message_service = MessageService::new(
            message_repo.clone(),
            chat_service.clone(),
            chat_validator.clone(),
            event_service.clone(),
        );

        Ok(Self {
            cfg,
            auth_service,
            user_service,
            chat_validator,
            chat_service,
            message_service,
            event_service,
        })
    }
}

impl FromRef<State> for integration::Config {
    fn from_ref(s: &State) -> integration::Config {
        s.cfg.clone()
    }
}

impl FromRef<State> for AuthService {
    fn from_ref(s: &State) -> AuthService {
        s.auth_service.clone()
    }
}

impl FromRef<State> for UserService {
    fn from_ref(s: &State) -> Self {
        s.user_service.clone()
    }
}

impl FromRef<State> for ChatValidator {
    fn from_ref(s: &State) -> Self {
        s.chat_validator.clone()
    }
}

impl FromRef<State> for ChatService {
    fn from_ref(s: &State) -> Self {
        s.chat_service.clone()
    }
}

impl FromRef<State> for MessageService {
    fn from_ref(s: &State) -> Self {
        s.message_service.clone()
    }
}

impl FromRef<State> for EventService {
    fn from_ref(s: &State) -> Self {
        s.event_service.clone()
    }
}
