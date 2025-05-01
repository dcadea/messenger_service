use axum::http::StatusCode;

impl From<super::Error> for StatusCode {
    fn from(e: super::Error) -> Self {
        match e {
            super::Error::Unauthorized => StatusCode::UNAUTHORIZED,
            super::Error::Forbidden | super::Error::UnknownKid | super::Error::InvalidState => {
                StatusCode::FORBIDDEN
            }
            super::Error::TokenMalformed => StatusCode::BAD_REQUEST,
            super::Error::_Configuration(_)
            | super::Error::_JsonWebtoken(_)
            | super::Error::_Uuid(_)
            | super::Error::_Reqwest(_)
            | super::Error::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub(super) mod pages {
    use crate::{auth::markup, markup::Wrappable};

    pub async fn login() -> Wrappable {
        Wrappable::new(markup::Login)
    }
}

pub(super) mod api {
    use crate::auth::{self, Code, Csrf};
    use axum::{
        extract::State,
        response::{IntoResponse, Redirect},
    };
    use axum_extra::extract::cookie::{self, Cookie};
    use axum_extra::extract::{CookieJar, Query};
    use log::debug;
    use serde::Deserialize;

    pub async fn sso_login(auth_service: State<auth::Service>) -> impl IntoResponse {
        Redirect::to(&auth_service.authorize().await)
    }

    pub async fn logout(
        auth_service: State<auth::Service>,
        jar: CookieJar,
    ) -> crate::Result<impl IntoResponse> {
        if let Some(sid) = jar.get(auth::SESSION_ID) {
            let sid = sid.value();
            debug!("Logging out user with sid: {}", &sid);
            auth_service.invalidate_token(sid).await?;
            return Ok((CookieJar::new(), Redirect::to("/login")));
        }

        debug!("No sid found, redirecting to login");
        Ok((jar, Redirect::to("/login")))
    }

    #[derive(Deserialize)]
    pub struct Params {
        code: Code,
        state: Csrf,
    }

    pub async fn callback(
        Query(params): Query<Params>,
        auth_service: State<auth::Service>,
        jar: CookieJar,
    ) -> crate::Result<impl IntoResponse> {
        let (token, ttl) = auth_service
            .exchange_code(params.code, params.state)
            .await?;

        let sid = uuid::Uuid::new_v4();
        auth_service.cache_token(&sid, token.secret(), &ttl).await;

        let sid = {
            let mut sid = Cookie::new(auth::SESSION_ID, sid.to_string());
            sid.set_secure(true);
            sid.set_http_only(true);
            sid.set_same_site(cookie::SameSite::Lax);
            sid
        };

        Ok((jar.add(sid), Redirect::to("/")))
    }
}
