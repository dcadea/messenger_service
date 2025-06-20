use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use log::debug;
use oauth2::AccessToken;

use crate::user::{self, model::UserDto};
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

    let user_dto = match user_service.find_by_sub(sub).await {
        Ok(u) => u,
        Err(user::Error::NotFound(_)) => {
            debug!("{sub:?} not projected, fetching from IdP");
            let user_info = auth_service.get_user_info(token.secret()).await?;
            let id = user_service.project(&user_info)?;
            UserDto::new(id, &user_info)
        }
        Err(e) => return Err(e.into()),
    };

    let auth_user = auth::User::from(user_dto);
    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}
