use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use oauth2::AccessToken;

use crate::auth;
use crate::user;

pub async fn validate_sid(
    auth_service: State<auth::Service>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> crate::Result<Response> {
    if let Some(sid) = jar.get(super::SESSION_ID) {
        match auth_service.find_token(sid.value()).await {
            Some(token) => {
                let sub = auth_service.validate(&token).await?;
                let ext = req.extensions_mut();
                ext.insert(sub);
                ext.insert(AccessToken::new(token));
            }
            None => return Ok(Redirect::to("/logout").into_response()),
        }
    }

    Ok(next.run(req).await)
}

pub async fn authorize(
    user_service: State<user::Service>,
    auth_service: State<auth::Service>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> crate::Result<Response> {
    if jar.get(super::SESSION_ID).is_none() {
        return Ok(Redirect::to("/login").into_response());
    }

    let ext = req.extensions();
    let sub: &user::Sub = ext.get().ok_or(super::Error::Unauthorized)?;
    let token: &AccessToken = ext.get().ok_or(super::Error::Unauthorized)?;

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

    req.extensions_mut().insert(user_info);

    Ok(next.run(req).await)
}
