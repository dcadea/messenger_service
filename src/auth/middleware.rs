use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use log::debug;
use oauth2::AccessToken;

use crate::user;
use crate::{
    auth::{self, Session},
    user::Sub,
};

pub async fn validate_sid(
    auth_service: State<auth::Service>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> crate::Result<Response> {
    if let Some(sid) = jar.get(super::Session::ID) {
        let sid = Session::from(sid);
        debug!("Active {sid:?} found");

        if let Some(token) = auth_service.find_token(&sid).await {
            let sub = auth_service.validate(&token).await?;
            let ext = req.extensions_mut();
            ext.insert(sub);
            ext.insert(AccessToken::new(token));
        } else {
            debug!("No associated token found for {sid:?}");
            return Ok(Redirect::to("/logout").into_response());
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
    if jar.get(super::Session::ID).is_none() {
        return Ok(Redirect::to("/login").into_response());
    }

    let ext = req.extensions();
    let sub: &Sub = ext.get().ok_or(super::Error::Unauthorized)?;
    let token: &AccessToken = ext.get().ok_or(super::Error::Unauthorized)?;

    let user_info = match user_service.find_one(sub).await {
        Ok(user_info) => user_info,
        Err(user::Error::NotFound(_)) => {
            debug!("{sub:?} not projected, fetching from IdP");
            let user_info = auth_service.get_user_info(token.secret()).await?;
            user_service.project(&user_info).await?;
            user_info
        }
        Err(e) => return Err(e.into()),
    };

    let auth_user = auth::User::from(user_info);
    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}
