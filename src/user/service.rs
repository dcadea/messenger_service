use std::sync::Arc;

use openid::{Bearer, Options, Token};
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
    pub async fn exists(&self, nickname: &str) -> bool {
        self.repository.find_one(nickname).await.is_some()
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
        let mut token: Token = bearer.clone().into();

        if let Some(id_token) = token.id_token.as_mut() {
            self.oidc_client.decode_token(id_token)?;
            self.oidc_client.validate_token(id_token, None, None)?;

            let userinfo = self.oidc_client.request_userinfo(&token).await?;
            if let Some(nickname) = userinfo.nickname.as_ref() {
                if !self.exists(nickname).await {
                    self.repository.insert(&userinfo.clone().into()).await?;
                }
            }
        }

        Ok(bearer)
    }
}
