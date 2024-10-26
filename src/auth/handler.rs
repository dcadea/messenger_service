use super::service::AuthService;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{self, Cookie};
use axum_extra::extract::{CookieJar, Query};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    code: String,
    state: String,
}

pub async fn login(auth_service: State<AuthService>) -> super::Result<impl IntoResponse> {
    let auth_url = auth_service.authorize().await?;
    Ok(Redirect::to(&auth_url))
}

pub async fn logout(
    auth_service: State<AuthService>,
    jar: CookieJar,
) -> super::Result<impl IntoResponse> {
    if let Some(sid) = jar.get(super::SESSION_ID) {
        auth_service.invalidate_token(sid.value()).await?;
        return Ok((CookieJar::new(), Redirect::to("/login")));
    }

    Ok((jar, Redirect::to("/login")))
}

pub async fn callback(
    params: Query<Params>,
    auth_service: State<AuthService>,
    jar: CookieJar,
) -> super::Result<impl IntoResponse> {
    let (token, ttl) = auth_service
        .exchange_code(&params.code, &params.state)
        .await?;

    let sid = uuid::Uuid::new_v4();
    auth_service.cache_token(&sid, token.secret(), &ttl).await?;

    let mut sid = Cookie::new(super::SESSION_ID, sid.to_string());
    sid.set_secure(true);
    sid.set_http_only(true);
    sid.set_same_site(cookie::SameSite::Lax);

    Ok((jar.add(sid), Redirect::to("/")))
}
