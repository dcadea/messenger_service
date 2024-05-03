use std::sync::Arc;

use openid::{Bearer, Options};
use url::Url;

use crate::integration::client::OpenIDClient;
use crate::result::Result;
use crate::user::repository::UserRepository;

pub struct UserService {
    repository: Arc<UserRepository>,
    oidc_client: Arc<OpenIDClient>,
}

impl UserService {
    pub fn new(repository: Arc<UserRepository>, oidc_client: Arc<OpenIDClient>) -> Arc<Self> {
        Self {
            repository,
            oidc_client,
        }
        .into()
    }
}

impl UserService {
    pub async fn exists(&self, username: &str) -> bool {
        self.repository.find_one(username).await.is_some()
    }
}

impl UserService {
    pub(super) fn authorize_url(&self) -> Url {
        self.oidc_client.auth_url(&Options {
            scope: Some("openid profile email".into()),
            ..Default::default()
        })
    }

    pub(super) async fn request_token(&self, code: &str) -> Result<Bearer> {
        let bearer = self.oidc_client.request_token(code).await?;
        Ok(bearer)
    }
}
