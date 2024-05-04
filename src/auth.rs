use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode, decode_header, DecodingKey, Header};
use serde_json::Value;

use crate::error::ApiError;
use crate::result::Result;
use crate::state::AppState;

pub async fn validate_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response> {
    let (token, header) = get_token(&headers)?;
    validate((token, &header), &state)?;

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

fn validate(token_header: (&str, &Header), state: &AppState) -> Result<()> {
    let jwk = token_header.1.kid.as_ref()
        .map(|kid| state.jwk_set.find(&kid))
        .flatten()
        .ok_or(ApiError::Forbidden)?;

    let decoding_key = DecodingKey::from_jwk(jwk)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    decode::<Value>(token_header.0, &decoding_key, &state.jwt_validator)
        .map(|_| ())
        .map_err(|_| ApiError::Forbidden)
}
