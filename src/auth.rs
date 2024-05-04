use axum::Extension;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode_header, Header};
use jsonwebtoken::jwk::JwkSet;
use serde_json::Value;
use crate::integration::Config;

use crate::state::AppState;

pub async fn validate_token(
    State(state): State<AppState>,
    Extension(config): Extension<Config>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match get_token(&headers) {
        Some((token, header)) if token_is_valid((token, &header), &state, &config).await => {
            let response = next.run(request).await;
            Ok(response)
        }
        _ => {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

// TODO: return Result<(&str, Header), ApiError>
fn get_token(headers: &HeaderMap) -> Option<(&str, Header)> {
    let auth_header = headers.get("Authorization")?.to_str().ok()?;
    let token = auth_header.split_whitespace().last()?;
    let decoded_header = decode_header(token).ok()?;
    Some((token, decoded_header))
}

// TODO: get rid of unwraps
async fn token_is_valid(token_header: (&str, &Header), state: &AppState, config: &Config) -> bool {
    let http = state.http.clone();
    let jwk_response = http.get(config.jwks_url.clone()).send().await.unwrap();
    let jwk_json = jwk_response.json().await.unwrap();
    let jwk_set: JwkSet = serde_json::from_value(jwk_json).unwrap();

    let kid = token_header.1.kid.as_ref().unwrap();
    let jwk = jwk_set.find(&kid).unwrap();

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&config.audience);
    let decoded = jsonwebtoken::decode::<Value>(
        token_header.0,
        &jsonwebtoken::DecodingKey::from_jwk(jwk).unwrap(),
        &validation,
    );

    decoded.is_ok()
}