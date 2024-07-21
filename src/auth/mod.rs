use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use thiserror::Error;

use self::model::TokenClaims;
use self::service::AuthService;

use crate::integration::IntegrationError;

use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::user::UserError;

pub mod service;

mod model;

type Result<T> = std::result::Result<T, AuthError>;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum AuthError {
    #[error("unauthorized to access the resource")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("missing or unknown kid")]
    UnknownKid,
    #[error("token is malformed: {0}")]
    TokenMalformed(String),
    #[error("unexpected auth error: {0}")]
    Unexpected(String),

    _UserError(#[from] UserError),
    _IntegrationError(#[from] IntegrationError),
    _ReqwestError(#[from] reqwest::Error),
    _ParseJsonError(#[from] serde_json::Error),
}

pub async fn validate_token(
    auth_service: State<AuthService>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request,
    next: Next,
) -> super::result::Result<Response> {
    let auth_header = auth_header.ok_or(AuthError::Unauthorized)?;
    let claims = auth_service.validate(auth_header.token()).await?;
    request.extensions_mut().insert(claims);

    let response = next.run(request).await;
    Ok(response)
}

pub async fn set_user_context(
    user_service: State<UserService>,
    auth_service: State<AuthService>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> super::result::Result<Response> {
    let claims = request
        .extensions()
        .get::<TokenClaims>()
        .ok_or(AuthError::Unauthorized)?;

    let user_info = match user_service.find_user_info(claims.sub.clone()).await {
        Ok(user_info) => user_info,
        Err(UserError::NotFound(_)) => {
            let user_info = auth_service.get_user_info(auth_header.token()).await?;
            let user = user_info.clone().into();
            user_service.create(&user).await?;
            user_info
        }
        Err(e) => return Err(e.into()),
    };

    request.extensions_mut().insert(user_info);

    let response = next.run(request).await;
    Ok(response)
}

pub async fn cache_user_friends(
    user_service: State<UserService>,
    request: Request,
    next: Next,
) -> super::result::Result<Response> {
    let user_info = request
        .extensions()
        .get::<UserInfo>()
        .ok_or(AuthError::Unauthorized)?;

    user_service.cache_friends(user_info.sub.clone()).await?;

    let response = next.run(request).await;
    Ok(response)
}
