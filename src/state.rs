use std::sync::Arc;

use axum::extract::FromRef;

use crate::auth::service::AuthServiceImpl;
use crate::contact::repository::MongoContactRepository;
use crate::contact::service::ContactServiceImpl;
use crate::event::service::EventServiceImpl;
use crate::integration::storage;
use crate::message::repository::MongoMessageRepository;
use crate::message::service::MessageServiceImpl;
use crate::talk::repository::MongoTalkRepository;
use crate::talk::service::{TalkServiceImpl, TalkValidatorImpl};
use crate::user::repository::PgUserRepository;
use crate::user::service::UserServiceImpl;
use crate::{auth, contact, event, message, talk, user};

use super::integration;

#[derive(Clone)]
pub struct AppState {
    cfg: integration::Config,

    auth_service: auth::Service,
    user_service: user::Service,
    contact_service: contact::Service,
    talk_service: talk::Service,
    talk_validator: talk::Validator,
    message_service: message::Service,
    event_service: event::Service,

    s3: storage::S3,
}

impl AppState {
    pub async fn init(cfg: integration::Config) -> crate::Result<Self> {
        let db = cfg.mongo().connect();
        let pg = cfg.pg().connect();
        let redis = cfg.redis().connect().await;
        let pubsub = cfg.pubsub().connect().await;
        let s3 = cfg.s3().connect().await;

        let auth_service = Arc::new(AuthServiceImpl::try_new(cfg.idp(), redis.clone()));
        let event_service = Arc::new(EventServiceImpl::new(pubsub));

        let contact_repo = Arc::new(MongoContactRepository::new(&db));
        let contact_service = Arc::new(ContactServiceImpl::new(contact_repo, redis.clone()));

        let user_repo = Arc::new(PgUserRepository::new(pg));
        let user_service = Arc::new(UserServiceImpl::new(
            user_repo,
            contact_service.clone(),
            event_service.clone(),
            redis.clone(),
        ));

        let talk_repo = Arc::new(MongoTalkRepository::new(&db));
        let message_repo = Arc::new(MongoMessageRepository::new(&db));

        let talk_validator = Arc::new(TalkValidatorImpl::new(talk_repo.clone(), redis.clone()));
        let talk_service = Arc::new(TalkServiceImpl::new(
            talk_repo,
            talk_validator.clone(),
            user_service.clone(),
            contact_service.clone(),
            event_service.clone(),
            message_repo.clone(),
            redis,
            s3.clone(),
        ));

        let message_service = Arc::new(MessageServiceImpl::new(
            message_repo,
            talk_service.clone(),
            talk_validator.clone(),
            event_service.clone(),
        ));

        Ok(Self {
            cfg,
            auth_service,
            user_service,
            contact_service,
            talk_service,
            talk_validator,
            message_service,
            event_service,
            s3,
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

impl FromRef<AppState> for contact::Service {
    fn from_ref(s: &AppState) -> Self {
        s.contact_service.clone()
    }
}

impl FromRef<AppState> for talk::Service {
    fn from_ref(s: &AppState) -> Self {
        s.talk_service.clone()
    }
}

impl FromRef<AppState> for talk::Validator {
    fn from_ref(s: &AppState) -> Self {
        s.talk_validator.clone()
    }
}

impl FromRef<AppState> for message::Service {
    fn from_ref(s: &AppState) -> Self {
        s.message_service.clone()
    }
}

impl FromRef<AppState> for event::Service {
    fn from_ref(s: &AppState) -> Self {
        s.event_service.clone()
    }
}

impl FromRef<AppState> for storage::S3 {
    fn from_ref(s: &AppState) -> Self {
        s.s3.clone()
    }
}
