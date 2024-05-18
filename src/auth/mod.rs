use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use crate::error::ApiError;
use crate::result::Result;
use crate::user::service::UserService;
use model::TokenClaims;
use service::AuthService;

pub mod model;
pub mod service;

pub async fn validate_token(
    auth_service: State<AuthService>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
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
) -> Result<Response> {
    let claims = request
        .extensions()
        .get::<TokenClaims>()
        .ok_or(ApiError::Unauthorized)?;

    let user_info = match user_service.find_by_sub(&claims.sub).await {
        Some(user) => user.into(),
        None => {
            let user_info = auth_service.get_user_info(auth_header.token()).await?;
            let user = user_info.clone().into();
            user_service.create(&user).await?;
            user_info
        }
    };

    request.extensions_mut().insert(user_info);

    let response = next.run(request).await;
    Ok(response)
}
