use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use self::model::TokenClaims;
use self::service::AuthService;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{integration, user};

pub mod service;

mod model;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
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

    _User(#[from] user::Error),
    _Integration(#[from] integration::Error),

    _Reqwest(#[from] reqwest::Error),
    _ParseJson(#[from] serde_json::Error),
}

pub async fn validate_token(
    auth_service: State<AuthService>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request,
    next: Next,
) -> super::result::Result<Response> {
    let auth_header = auth_header.ok_or(Error::Unauthorized)?;
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
        .ok_or(Error::Unauthorized)?;

    let user_info = match user_service.find_user_info(claims.sub.clone()).await {
        Ok(user_info) => user_info,
        Err(user::Error::NotFound(_)) => {
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
        .ok_or(Error::Unauthorized)?;

    user_service.cache_friends(user_info.sub.clone()).await?;

    let response = next.run(request).await;
    Ok(response)
}
