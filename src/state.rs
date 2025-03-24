use std::sync::Arc;

use axum::extract::FromRef;

use crate::auth;
use crate::auth::service::AuthServiceImpl;
use crate::talk::repository::TalkRepository;
use crate::talk::service::{TalkService, TalkValidator};

use super::event::service::EventService;
use super::integration;
use super::message::repository::MessageRepository;
use super::message::service::MessageService;
use super::user::repository::UserRepository;
use super::user::service::UserService;

#[derive(Clone)]
pub struct AppState {
    cfg: integration::Config,

    auth_service: auth::Service,
    user_service: UserService,
    talk_service: TalkService,
    talk_validator: TalkValidator,
    message_service: MessageService,
    event_service: EventService,
}

impl AppState {
    pub async fn init(cfg: integration::Config) -> crate::Result<Self> {
        let db = cfg.mongo.connect();
        let redis = cfg.redis.connect().await;
        let pubsub = cfg.pubsub.connect().await;

        let auth_service = AuthServiceImpl::try_new(&cfg.idp, redis.clone())?;
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
            auth_service: Arc::new(auth_service),
            user_service,
            talk_service,
            talk_validator,
            message_service,
            event_service,
        })
    }
}

impl FromRef<AppState> for integration::Config {
    fn from_ref(s: &AppState) -> integration::Config {
        s.cfg.clone()
    }
}

impl FromRef<AppState> for auth::Service {
    fn from_ref(s: &AppState) -> auth::Service {
        s.auth_service.clone()
    }
}

impl FromRef<AppState> for UserService {
    fn from_ref(s: &AppState) -> Self {
        s.user_service.clone()
    }
}

impl FromRef<AppState> for TalkService {
    fn from_ref(s: &AppState) -> Self {
        s.talk_service.clone()
    }
}

impl FromRef<AppState> for TalkValidator {
    fn from_ref(s: &AppState) -> Self {
        s.talk_validator.clone()
    }
}

impl FromRef<AppState> for MessageService {
    fn from_ref(s: &AppState) -> Self {
        s.message_service.clone()
    }
}

impl FromRef<AppState> for EventService {
    fn from_ref(s: &AppState) -> Self {
        s.event_service.clone()
    }
}
