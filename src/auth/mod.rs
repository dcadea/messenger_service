use self::service::AuthService;
use crate::markup::wrap_in_base;
use crate::state::AppState;
use crate::user::model::UserInfo;
use crate::user::service::UserService;
use crate::{integration, user};
use axum::extract::{Request, State};
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum_extra::extract::CookieJar;
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use oauth2::AccessToken;
use serde::Deserialize;

mod handler;
mod markup;
pub(crate) mod service;

type Result<T> = std::result::Result<T, Error>;

const SESSION_ID: &str = "session_id";

#[derive(Deserialize, Clone)]
struct TokenClaims {
    sub: user::Sub,
}

pub(crate) fn pages<S>(state: AppState) -> Router<S> {
    Router::new()
        .route(
            "/login",
            get(markup::login).route_layer(from_fn(wrap_in_base)),
        )
        .with_state(state)
}

pub(crate) fn endpoints<S>(state: AppState) -> Router<S> {
    Router::new()
        .route("/sso/login", get(handler::login))
        .route("/logout", get(handler::logout))
        .route("/callback", get(handler::callback))
        .with_state(state)
}

pub(crate) async fn validate_token(
    auth_service: State<AuthService>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> crate::Result<Response> {
    let token = match auth_header {
        Some(ah) => Ok(ah.token().into()),
        None => match jar.get(SESSION_ID) {
            Some(sid) => auth_service
                .find_token(sid.value())
                .await
                .ok_or(Error::Unauthorized),
            None => Err(Error::Unauthorized),
        },
    }?;

    let sub = auth_service.validate(&token).await?;
    request.extensions_mut().insert(sub);
    request.extensions_mut().insert(AccessToken::new(token));

    let response = next.run(request).await;
    Ok(response)
}

pub(crate) async fn set_user_context(
    user_service: State<UserService>,
    auth_service: State<AuthService>,
    mut request: Request,
    next: Next,
) -> crate::Result<Response> {
    let sub = request
        .extensions()
        .get::<user::Sub>()
        .ok_or(Error::Unauthorized)?;

    let token = request
        .extensions()
        .get::<AccessToken>()
        .ok_or(Error::Unauthorized)?;

    let user_info = match user_service.find_user_info(&sub).await {
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

pub(crate) async fn cache_user_friends(
    user_service: State<UserService>,
    request: Request,
    next: Next,
) -> crate::Result<Response> {
    let user_info = request
        .extensions()
        .get::<UserInfo>()
        .ok_or(Error::Unauthorized)?;

    user_service.cache_friends(&user_info.sub).await?;

    let response = next.run(request).await;
    Ok(response)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    #[error("unauthorized to access the resource")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("missing or unknown kid")]
    UnknownKid,
    #[error("token is malformed: {0}")]
    TokenMalformed(String),
    #[error("unexpected auth error: {0}")]
    Unexpected(String),

    _User(#[from] user::Error),
    _Integration(#[from] integration::Error),

    _Reqwest(#[from] reqwest::Error),
    _ParseJson(#[from] serde_json::Error),
    _Redis(#[from] redis::RedisError),
}
