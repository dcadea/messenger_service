use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use oauth2::AccessToken;

use crate::user::{self, service::UserService};

use super::service::AuthService;

pub async fn validate_sid(
    auth_service: State<AuthService>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> crate::Result<Response> {
    if let Some(sid) = jar.get(super::SESSION_ID) {
        let token = auth_service
            .find_token(sid.value())
            .await
            .ok_or(super::Error::Forbidden(String::from("invalid sid")))?;

        let sub = auth_service.validate(&token).await?;
        request.extensions_mut().insert(sub);
        request.extensions_mut().insert(AccessToken::new(token));
    }

    Ok(next.run(request).await)
}

pub async fn authorize(
    user_service: State<UserService>,
    auth_service: State<AuthService>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> crate::Result<Response> {
    if jar.get(super::SESSION_ID).is_none() {
        return Ok(Redirect::to("/login").into_response());
    }

    let sub = request
        .extensions()
        .get::<user::Sub>()
        .ok_or(super::Error::Unauthorized)?;

    let token = request
        .extensions()
        .get::<AccessToken>()
        .ok_or(super::Error::Unauthorized)?;

    let user_info = match user_service.find_user_info(sub).await {
        Ok(user_info) => user_info,
        Err(user::Error::NotFound(_)) => {
            let user_info = auth_service.get_user_info(token.secret()).await?;
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
