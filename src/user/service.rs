use openid::Bearer;
use std::sync::Arc;

use crate::error::ApiError;
use crate::integration::client::OpenIDClient;
use crate::result::Result;
use crate::user::model::User;
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
    pub(super) async fn create(&self, user: &User) -> Result<()> {
        self.repository.insert(user).await
    }

    pub(super) async fn matches(&self, username: &str, password: &str) -> Result<()> {
        match self.repository.find_one(username).await {
            Some(user) => {
                if user.password.eq(password) {
                    return Ok(());
                }

                Err(ApiError::InvalidCredentials)
            }
            None => Err(ApiError::UserNotFound),
        }
    }

    pub async fn exists(&self, username: &str) -> bool {
        self.repository.find_one(username).await.is_some()
    }
}

impl UserService {
    pub(super) async fn request_token(&self, code: &str) -> Result<Bearer> {
        let bearer = self.oidc_client.request_token(code).await?;
        Ok(bearer)
    }
}
