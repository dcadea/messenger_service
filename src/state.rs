use axum::extract::FromRef;

use crate::talk::repository::TalkRepository;
use crate::talk::service::{TalkService, TalkValidator};

use super::auth::service::AuthService;
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
    pub talk_service: TalkService,
    pub talk_validator: TalkValidator,
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

        let talk_repo = TalkRepository::new(&db);
        let message_repo = MessageRepository::new(&db);

        let talk_validator = TalkValidator::new(talk_repo.clone(), redis.clone());
        let talk_service = TalkService::new(
            talk_repo.clone(),
            talk_validator.clone(),
            user_service.clone(),
            event_service.clone(),
            message_repo.clone(),
            redis.clone(),
        );

        let message_service = MessageService::new(
            message_repo.clone(),
            talk_service.clone(),
            talk_validator.clone(),
            event_service.clone(),
        );

        Ok(Self {
            cfg,
            auth_service,
            user_service,
            talk_service,
            talk_validator,
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

impl FromRef<State> for TalkService {
    fn from_ref(s: &State) -> Self {
        s.talk_service.clone()
    }
}

impl FromRef<State> for TalkValidator {
    fn from_ref(s: &State) -> Self {
        s.talk_validator.clone()
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
