use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode, decode_header, Header};
use serde_json::Value;

use crate::error::ApiError;
use crate::result::Result;
use crate::state::{AuthState};

pub async fn validate_token(
    State(state): State<AuthState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response> {
    let (token, header) = get_token(&headers)?;
    validate((token, &header), &state).await?;

    let response = next.run(request).await;
    Ok(response)
}

fn get_token(headers: &HeaderMap) -> Result<(&str, Header)> {
    let auth_header = headers.get("Authorization").ok_or(ApiError::Unauthorized)?;
    let bearer_token = auth_header.to_str().map_err(|e| ApiError::TokenMalformed(e.to_string()))?;
    let token = bearer_token.strip_prefix("Bearer ").ok_or(ApiError::Unauthorized)?;
    let decoded_header = decode_header(token).map_err(|e| ApiError::TokenMalformed(e.to_string()))?;
    Ok((token, decoded_header))
}

async fn validate(token_header: (&str, &Header), state: &AuthState) -> Result<()> {
    let kid = token_header.1.kid.as_ref().ok_or(ApiError::Forbidden)?;
    let decoding_keys_guard = state.jwk_decoding_keys.lock().await;
    let decoding_key = decoding_keys_guard.get(kid)
        .ok_or(ApiError::Forbidden)?;

    decode::<Value>(token_header.0, &decoding_key, &state.jwt_validator)
        .map(|_| ())
        .map_err(|_| ApiError::Forbidden)
}
