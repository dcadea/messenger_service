use std::sync::Arc;

use axum::extract::FromRef;

use crate::auth::service::AuthServiceImpl;
use crate::message::repository::MongoMessageRepository;
use crate::message::service::MessageServiceImpl;
use crate::talk::repository::MongoTalkRepository;
use crate::talk::service::{TalkServiceImpl, TalkValidator};
use crate::user::repository::MongoUserRepository;
use crate::user::service::UserServiceImpl;
use crate::{auth, message, talk, user};

use super::event::service::EventService;
use super::integration;

#[derive(Clone)]
pub struct AppState {
    cfg: integration::Config,

    auth_service: auth::Service,
    user_service: user::Service,
    talk_service: talk::Service,
    talk_validator: TalkValidator,
    message_service: message::Service,
    event_service: EventService,
}

impl AppState {
    pub async fn init(cfg: integration::Config) -> crate::Result<Self> {
        let db = cfg.mongo.connect();
        let redis = cfg.redis.connect().await;
        let pubsub = cfg.pubsub.connect().await;

        let auth_service = Arc::new(AuthServiceImpl::try_new(&cfg.idp, redis.clone())?);
        let event_service = EventService::new(pubsub);

        let user_repo = Arc::new(MongoUserRepository::new(&db));
        let user_service = Arc::new(UserServiceImpl::new(
            user_repo,
            event_service.clone(),
            redis.clone(),
        ));

        let talk_repo = Arc::new(MongoTalkRepository::new(&db));
        let message_repo = Arc::new(MongoMessageRepository::new(&db));

        let talk_validator = TalkValidator::new(talk_repo.clone(), redis.clone());
        let talk_service = Arc::new(TalkServiceImpl::new(
            talk_repo.clone(),
            talk_validator.clone(),
            user_service.clone(),
            event_service.clone(),
            message_repo.clone(),
            redis.clone(),
        ));

        let message_service = Arc::new(MessageServiceImpl::new(
            message_repo.clone(),
            talk_service.clone(),
            talk_validator.clone(),
            event_service.clone(),
        ));

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

impl FromRef<AppState> for integration::Config {
    fn from_ref(s: &AppState) -> Self {
        s.cfg.clone()
    }
}

impl FromRef<AppState> for auth::Service {
    fn from_ref(s: &AppState) -> Self {
        s.auth_service.clone()
    }
}

impl FromRef<AppState> for user::Service {
    fn from_ref(s: &AppState) -> Self {
        s.user_service.clone()
    }
}

impl FromRef<AppState> for talk::Service {
    fn from_ref(s: &AppState) -> Self {
        s.talk_service.clone()
    }
}

impl FromRef<AppState> for TalkValidator {
    fn from_ref(s: &AppState) -> Self {
        s.talk_validator.clone()
    }
}

impl FromRef<AppState> for message::Service {
    fn from_ref(s: &AppState) -> Self {
        s.message_service.clone()
    }
}

impl FromRef<AppState> for EventService {
    fn from_ref(s: &AppState) -> Self {
        s.event_service.clone()
    }
}
