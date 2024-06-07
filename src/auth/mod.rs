use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use crate::auth::error::AuthError;
use crate::user::error::UserError;
use crate::user::service::UserService;
use model::TokenClaims;
use service::AuthService;

pub mod error;
mod model;
pub mod service;

type Result<T> = std::result::Result<T, AuthError>;

pub async fn validate_token(
    auth_service: State<AuthService>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> super::result::Result<Response> {
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

    let user_info = match user_service.find_user_info(&claims.sub).await {
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
