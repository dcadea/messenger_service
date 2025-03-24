pub(super) mod pages {
    use crate::auth::markup;
    use messenger_service::markup::Wrappable;

    pub async fn login() -> Wrappable {
        Wrappable::new(markup::Login)
    }
}

pub(super) mod api {
    use crate::auth;
    use axum::{
        extract::State,
        response::{IntoResponse, Redirect},
    };
    use axum_extra::extract::cookie::{self, Cookie};
    use axum_extra::extract::{CookieJar, Query};
    use serde::Deserialize;

    pub async fn sso_login(auth_service: State<auth::Service>) -> impl IntoResponse {
        Redirect::to(&auth_service.authorize().await)
    }

    pub async fn logout(
        auth_service: State<auth::Service>,
        jar: CookieJar,
    ) -> crate::Result<impl IntoResponse> {
        if let Some(sid) = jar.get(auth::SESSION_ID) {
            auth_service.invalidate_token(sid.value()).await?;
            return Ok((CookieJar::new(), Redirect::to("/login")));
        }

        Ok((jar, Redirect::to("/login")))
    }

    #[derive(Deserialize)]
    pub struct Params {
        code: String,
        state: String,
    }

    pub async fn callback(
        params: Query<Params>,
        auth_service: State<auth::Service>,
        jar: CookieJar,
    ) -> crate::Result<impl IntoResponse> {
        let (token, ttl) = auth_service
            .exchange_code(&params.code, &params.state)
            .await?;

        let sid = uuid::Uuid::new_v4();
        auth_service.cache_token(&sid, token.secret(), &ttl).await;

        let mut sid = Cookie::new(auth::SESSION_ID, sid.to_string());
        sid.set_secure(true);
        sid.set_http_only(true);
        sid.set_same_site(cookie::SameSite::Lax);

        Ok((jar.add(sid), Redirect::to("/")))
    }
}
