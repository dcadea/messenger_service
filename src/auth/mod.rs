use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::model::TokenClaims;
use crate::auth::service::AuthService;
use crate::error::ApiError;
use crate::result::Result;
use crate::user::service::UserService;

pub(crate) mod model;
pub(crate) mod service;

const AUTHORIZATION: &str = "Authorization";

pub(super) async fn validate_token(
    auth_service: State<AuthService>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let token = get_token(&headers)?;

    let claims = auth_service.validate(token).await?;
    request.extensions_mut().insert(claims);

    let response = next.run(request).await;
    Ok(response)
}

pub(crate) async fn set_user_context(
    user_service: State<UserService>,
    auth_service: State<AuthService>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let claims = request
        .extensions()
        .get::<TokenClaims>()
        .ok_or(ApiError::Unauthorized)?;

    let user = match user_service.find_by_sub(&claims.sub).await {
        Some(user) => user,
        None => {
            let token = get_token(&headers)?;
            let user_info = auth_service.get_user_info(token).await?;
            let user = user_info.into();
            user_service.create(&user).await?;
            user
        }
    };

    request.extensions_mut().insert(user);

    let response = next.run(request).await;
    Ok(response)
}

fn get_token(headers: &HeaderMap) -> Result<&str> {
    let auth_header = headers.get(AUTHORIZATION).ok_or(ApiError::Unauthorized)?;
    let bearer_token = auth_header
        .to_str()
        .map_err(|e| ApiError::TokenMalformed(e.to_string()))?;
    let token = bearer_token
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;
    Ok(token)
}
