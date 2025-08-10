use std::sync::Arc;

use axum::extract::FromRef;

use crate::auth::service::AuthServiceImpl;
use crate::contact::repository::PgContactRepository;
use crate::contact::service::ContactServiceImpl;
use crate::event::service::EventServiceImpl;
use crate::message::repository::PgMessageRepository;
use crate::message::service::MessageServiceImpl;
use crate::talk::repository::PgTalkRepository;
use crate::talk::service::TalkServiceImpl;
use crate::user::repository::PgUserRepository;
use crate::user::service::UserServiceImpl;
use crate::{auth, contact, event, message, talk, user};

use super::integration;

#[derive(Clone)]
pub struct AppServices {
    auth: auth::Service,
    user: user::Service,
    contact: contact::Service,
    talk: talk::Service,
    message: message::Service,
    event: event::Service,
}

impl AppServices {
    pub async fn init(cfg: integration::Config) -> Self {
        let pg = cfg.pg().connect();
        let redis = cfg.redis().connect().await;
        let pubsub = cfg.pubsub().connect().await;
        let s3 = cfg.s3().connect().await;

        let auth_service = Arc::new(AuthServiceImpl::new(cfg.idp(), redis.clone()));
        let event_service = Arc::new(EventServiceImpl::new(pubsub));

        let contact_repo = Arc::new(PgContactRepository::new(pg.clone()));
        let contact_service = Arc::new(ContactServiceImpl::new(contact_repo, redis.clone()));

        let user_repo = Arc::new(PgUserRepository::new(pg.clone()));
        let user_service = Arc::new(UserServiceImpl::new(
            user_repo,
            contact_service.clone(),
            event_service.clone(),
            redis.clone(),
        ));

        let talk_repo = Arc::new(PgTalkRepository::new(pg.clone()));
        let message_repo = Arc::new(PgMessageRepository::new(pg));

        let talk_service = Arc::new(TalkServiceImpl::new(
            talk_repo,
            user_service.clone(),
            contact_service.clone(),
            event_service.clone(),
            redis.clone(),
            s3,
        ));

        let message_service = Arc::new(MessageServiceImpl::new(
            message_repo,
            user_service.clone(),
            event_service.clone(),
        ));

        Self {
            auth: auth_service,
            user: user_service,
            contact: contact_service,
            talk: talk_service,
            message: message_service,
            event: event_service,
        }
    }
}

impl FromRef<AppServices> for auth::Service {
    fn from_ref(s: &AppServices) -> Self {
        s.auth.clone()
    }
}

impl FromRef<AppServices> for user::Service {
    fn from_ref(s: &AppServices) -> Self {
        s.user.clone()
    }
}

impl FromRef<AppServices> for contact::Service {
    fn from_ref(s: &AppServices) -> Self {
        s.contact.clone()
    }
}

impl FromRef<AppServices> for talk::Service {
    fn from_ref(s: &AppServices) -> Self {
        s.talk.clone()
    }
}

impl FromRef<AppServices> for message::Service {
    fn from_ref(s: &AppServices) -> Self {
        s.message.clone()
    }
}

impl FromRef<AppServices> for event::Service {
    fn from_ref(s: &AppServices) -> Self {
        s.event.clone()
    }
}
