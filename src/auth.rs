use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode, decode_header, Header};

use crate::error::ApiError;
use crate::result::Result;
use crate::state::{AppState, AuthState};
use crate::user::model::{TokenClaims, User};

const AUTHORIZATION: &str = "Authorization";

pub async fn validate_token(
    auth_state: State<AuthState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let token = get_token(&headers)?;
    let jwt_header = get_jwt_header(token)?;

    let claims = validate(token, &jwt_header, &auth_state).await?;
    request.extensions_mut().insert(claims);

    let response = next.run(request).await;
    Ok(response)
}

pub async fn set_user_context(
    app_state: State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let claims = request
        .extensions()
        .get::<TokenClaims>()
        .ok_or(ApiError::Unauthorized)?;

    let user = match app_state.user_service.find_by_sub(&claims.sub).await {
        Some(user) => user,
        None => {
            let user = app_state
                .http
                .get(&app_state.config.userinfo_url)
                .header(
                    AUTHORIZATION,
                    headers.get(AUTHORIZATION).ok_or(ApiError::Unauthorized)?,
                )
                .send()
                .await?
                .json::<User>()
                .await?;
            app_state.user_service.create(&user).await?;
            user
        }
    };

    request.extensions_mut().insert(user);

    let response = next.run(request).await;
    Ok(response)
}

fn get_token(headers: &HeaderMap) -> Result<&str> {
    let auth_header = headers.get(AUTHORIZATION).ok_or(ApiError::Unauthorized)?;
    let bearer_token = auth_header
        .to_str()
        .map_err(|e| ApiError::TokenMalformed(e.to_string()))?;
    let token = bearer_token
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;
    Ok(token)
}

fn get_jwt_header(token: &str) -> Result<Header> {
    let jwt_header = decode_header(token).map_err(|e| ApiError::TokenMalformed(e.to_string()))?;
    Ok(jwt_header)
}

async fn validate(token: &str, jwt_header: &Header, auth_state: &AuthState) -> Result<TokenClaims> {
    let kid = jwt_header.kid.as_ref().ok_or(ApiError::Forbidden("Missing kid".to_owned()))?;
    let decoding_keys_guard = auth_state.jwk_decoding_keys.lock().await;
    let decoding_key = decoding_keys_guard.get(kid).ok_or(ApiError::Forbidden("Unknown kid".to_owned()))?;

    decode::<TokenClaims>(token, &decoding_key, &auth_state.jwt_validator)
        .map(|data| data.claims)
        .map_err(|e| ApiError::Forbidden(e.to_string()))
}
